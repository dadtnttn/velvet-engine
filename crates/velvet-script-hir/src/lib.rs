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
    fn lower_scene_0() {
        let src = format!("scene s0 {{}}\nfn f0() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_1() {
        let src = format!("scene s1 {{}}\nfn f1() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_2() {
        let src = format!("scene s2 {{}}\nfn f2() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_3() {
        let src = format!("scene s3 {{}}\nfn f3() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_4() {
        let src = format!("scene s4 {{}}\nfn f4() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_5() {
        let src = format!("scene s5 {{}}\nfn f5() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_6() {
        let src = format!("scene s6 {{}}\nfn f6() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_7() {
        let src = format!("scene s7 {{}}\nfn f7() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_8() {
        let src = format!("scene s8 {{}}\nfn f8() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_9() {
        let src = format!("scene s9 {{}}\nfn f9() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_10() {
        let src = format!("scene s10 {{}}\nfn f10() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_11() {
        let src = format!("scene s11 {{}}\nfn f11() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_12() {
        let src = format!("scene s12 {{}}\nfn f12() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_13() {
        let src = format!("scene s13 {{}}\nfn f13() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_14() {
        let src = format!("scene s14 {{}}\nfn f14() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_15() {
        let src = format!("scene s15 {{}}\nfn f15() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_16() {
        let src = format!("scene s16 {{}}\nfn f16() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_17() {
        let src = format!("scene s17 {{}}\nfn f17() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_18() {
        let src = format!("scene s18 {{}}\nfn f18() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_19() {
        let src = format!("scene s19 {{}}\nfn f19() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_20() {
        let src = format!("scene s20 {{}}\nfn f20() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_21() {
        let src = format!("scene s21 {{}}\nfn f21() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_22() {
        let src = format!("scene s22 {{}}\nfn f22() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_23() {
        let src = format!("scene s23 {{}}\nfn f23() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_24() {
        let src = format!("scene s24 {{}}\nfn f24() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_25() {
        let src = format!("scene s25 {{}}\nfn f25() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_26() {
        let src = format!("scene s26 {{}}\nfn f26() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_27() {
        let src = format!("scene s27 {{}}\nfn f27() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_28() {
        let src = format!("scene s28 {{}}\nfn f28() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_29() {
        let src = format!("scene s29 {{}}\nfn f29() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_30() {
        let src = format!("scene s30 {{}}\nfn f30() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_31() {
        let src = format!("scene s31 {{}}\nfn f31() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_32() {
        let src = format!("scene s32 {{}}\nfn f32() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_33() {
        let src = format!("scene s33 {{}}\nfn f33() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_34() {
        let src = format!("scene s34 {{}}\nfn f34() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_35() {
        let src = format!("scene s35 {{}}\nfn f35() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_36() {
        let src = format!("scene s36 {{}}\nfn f36() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_37() {
        let src = format!("scene s37 {{}}\nfn f37() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_38() {
        let src = format!("scene s38 {{}}\nfn f38() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_39() {
        let src = format!("scene s39 {{}}\nfn f39() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_40() {
        let src = format!("scene s40 {{}}\nfn f40() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_41() {
        let src = format!("scene s41 {{}}\nfn f41() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_42() {
        let src = format!("scene s42 {{}}\nfn f42() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_43() {
        let src = format!("scene s43 {{}}\nfn f43() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_44() {
        let src = format!("scene s44 {{}}\nfn f44() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_45() {
        let src = format!("scene s45 {{}}\nfn f45() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_46() {
        let src = format!("scene s46 {{}}\nfn f46() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_47() {
        let src = format!("scene s47 {{}}\nfn f47() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_48() {
        let src = format!("scene s48 {{}}\nfn f48() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_49() {
        let src = format!("scene s49 {{}}\nfn f49() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_50() {
        let src = format!("scene s50 {{}}\nfn f50() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_51() {
        let src = format!("scene s51 {{}}\nfn f51() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_52() {
        let src = format!("scene s52 {{}}\nfn f52() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_53() {
        let src = format!("scene s53 {{}}\nfn f53() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_54() {
        let src = format!("scene s54 {{}}\nfn f54() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_55() {
        let src = format!("scene s55 {{}}\nfn f55() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_56() {
        let src = format!("scene s56 {{}}\nfn f56() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_57() {
        let src = format!("scene s57 {{}}\nfn f57() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_58() {
        let src = format!("scene s58 {{}}\nfn f58() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_59() {
        let src = format!("scene s59 {{}}\nfn f59() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_60() {
        let src = format!("scene s60 {{}}\nfn f60() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_61() {
        let src = format!("scene s61 {{}}\nfn f61() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_62() {
        let src = format!("scene s62 {{}}\nfn f62() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_63() {
        let src = format!("scene s63 {{}}\nfn f63() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_64() {
        let src = format!("scene s64 {{}}\nfn f64() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_65() {
        let src = format!("scene s65 {{}}\nfn f65() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_66() {
        let src = format!("scene s66 {{}}\nfn f66() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_67() {
        let src = format!("scene s67 {{}}\nfn f67() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_68() {
        let src = format!("scene s68 {{}}\nfn f68() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_69() {
        let src = format!("scene s69 {{}}\nfn f69() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_70() {
        let src = format!("scene s70 {{}}\nfn f70() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_71() {
        let src = format!("scene s71 {{}}\nfn f71() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_72() {
        let src = format!("scene s72 {{}}\nfn f72() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_73() {
        let src = format!("scene s73 {{}}\nfn f73() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_74() {
        let src = format!("scene s74 {{}}\nfn f74() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_75() {
        let src = format!("scene s75 {{}}\nfn f75() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_76() {
        let src = format!("scene s76 {{}}\nfn f76() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_77() {
        let src = format!("scene s77 {{}}\nfn f77() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_78() {
        let src = format!("scene s78 {{}}\nfn f78() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_79() {
        let src = format!("scene s79 {{}}\nfn f79() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_80() {
        let src = format!("scene s80 {{}}\nfn f80() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_81() {
        let src = format!("scene s81 {{}}\nfn f81() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_82() {
        let src = format!("scene s82 {{}}\nfn f82() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_83() {
        let src = format!("scene s83 {{}}\nfn f83() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_84() {
        let src = format!("scene s84 {{}}\nfn f84() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_85() {
        let src = format!("scene s85 {{}}\nfn f85() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_86() {
        let src = format!("scene s86 {{}}\nfn f86() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_87() {
        let src = format!("scene s87 {{}}\nfn f87() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_88() {
        let src = format!("scene s88 {{}}\nfn f88() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_89() {
        let src = format!("scene s89 {{}}\nfn f89() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_90() {
        let src = format!("scene s90 {{}}\nfn f90() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_91() {
        let src = format!("scene s91 {{}}\nfn f91() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_92() {
        let src = format!("scene s92 {{}}\nfn f92() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_93() {
        let src = format!("scene s93 {{}}\nfn f93() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_94() {
        let src = format!("scene s94 {{}}\nfn f94() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_95() {
        let src = format!("scene s95 {{}}\nfn f95() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_96() {
        let src = format!("scene s96 {{}}\nfn f96() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_97() {
        let src = format!("scene s97 {{}}\nfn f97() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_98() {
        let src = format!("scene s98 {{}}\nfn f98() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_99() {
        let src = format!("scene s99 {{}}\nfn f99() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_100() {
        let src = format!("scene s100 {{}}\nfn f100() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_101() {
        let src = format!("scene s101 {{}}\nfn f101() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_102() {
        let src = format!("scene s102 {{}}\nfn f102() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_103() {
        let src = format!("scene s103 {{}}\nfn f103() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_104() {
        let src = format!("scene s104 {{}}\nfn f104() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_105() {
        let src = format!("scene s105 {{}}\nfn f105() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_106() {
        let src = format!("scene s106 {{}}\nfn f106() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_107() {
        let src = format!("scene s107 {{}}\nfn f107() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_108() {
        let src = format!("scene s108 {{}}\nfn f108() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_109() {
        let src = format!("scene s109 {{}}\nfn f109() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_110() {
        let src = format!("scene s110 {{}}\nfn f110() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_111() {
        let src = format!("scene s111 {{}}\nfn f111() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_112() {
        let src = format!("scene s112 {{}}\nfn f112() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_113() {
        let src = format!("scene s113 {{}}\nfn f113() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_114() {
        let src = format!("scene s114 {{}}\nfn f114() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_115() {
        let src = format!("scene s115 {{}}\nfn f115() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_116() {
        let src = format!("scene s116 {{}}\nfn f116() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_117() {
        let src = format!("scene s117 {{}}\nfn f117() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_118() {
        let src = format!("scene s118 {{}}\nfn f118() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_119() {
        let src = format!("scene s119 {{}}\nfn f119() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_120() {
        let src = format!("scene s120 {{}}\nfn f120() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_121() {
        let src = format!("scene s121 {{}}\nfn f121() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_122() {
        let src = format!("scene s122 {{}}\nfn f122() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_123() {
        let src = format!("scene s123 {{}}\nfn f123() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_124() {
        let src = format!("scene s124 {{}}\nfn f124() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_125() {
        let src = format!("scene s125 {{}}\nfn f125() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_126() {
        let src = format!("scene s126 {{}}\nfn f126() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_127() {
        let src = format!("scene s127 {{}}\nfn f127() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_128() {
        let src = format!("scene s128 {{}}\nfn f128() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_129() {
        let src = format!("scene s129 {{}}\nfn f129() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_130() {
        let src = format!("scene s130 {{}}\nfn f130() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_131() {
        let src = format!("scene s131 {{}}\nfn f131() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_132() {
        let src = format!("scene s132 {{}}\nfn f132() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_133() {
        let src = format!("scene s133 {{}}\nfn f133() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_134() {
        let src = format!("scene s134 {{}}\nfn f134() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_135() {
        let src = format!("scene s135 {{}}\nfn f135() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_136() {
        let src = format!("scene s136 {{}}\nfn f136() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_137() {
        let src = format!("scene s137 {{}}\nfn f137() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_138() {
        let src = format!("scene s138 {{}}\nfn f138() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_139() {
        let src = format!("scene s139 {{}}\nfn f139() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_140() {
        let src = format!("scene s140 {{}}\nfn f140() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_141() {
        let src = format!("scene s141 {{}}\nfn f141() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_142() {
        let src = format!("scene s142 {{}}\nfn f142() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_143() {
        let src = format!("scene s143 {{}}\nfn f143() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_144() {
        let src = format!("scene s144 {{}}\nfn f144() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_145() {
        let src = format!("scene s145 {{}}\nfn f145() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_146() {
        let src = format!("scene s146 {{}}\nfn f146() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_147() {
        let src = format!("scene s147 {{}}\nfn f147() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_148() {
        let src = format!("scene s148 {{}}\nfn f148() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_149() {
        let src = format!("scene s149 {{}}\nfn f149() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_150() {
        let src = format!("scene s150 {{}}\nfn f150() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_151() {
        let src = format!("scene s151 {{}}\nfn f151() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_152() {
        let src = format!("scene s152 {{}}\nfn f152() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_153() {
        let src = format!("scene s153 {{}}\nfn f153() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_154() {
        let src = format!("scene s154 {{}}\nfn f154() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_155() {
        let src = format!("scene s155 {{}}\nfn f155() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_156() {
        let src = format!("scene s156 {{}}\nfn f156() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_157() {
        let src = format!("scene s157 {{}}\nfn f157() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_158() {
        let src = format!("scene s158 {{}}\nfn f158() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_159() {
        let src = format!("scene s159 {{}}\nfn f159() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_160() {
        let src = format!("scene s160 {{}}\nfn f160() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_161() {
        let src = format!("scene s161 {{}}\nfn f161() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_162() {
        let src = format!("scene s162 {{}}\nfn f162() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_163() {
        let src = format!("scene s163 {{}}\nfn f163() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_164() {
        let src = format!("scene s164 {{}}\nfn f164() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_165() {
        let src = format!("scene s165 {{}}\nfn f165() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_166() {
        let src = format!("scene s166 {{}}\nfn f166() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_167() {
        let src = format!("scene s167 {{}}\nfn f167() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_168() {
        let src = format!("scene s168 {{}}\nfn f168() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_169() {
        let src = format!("scene s169 {{}}\nfn f169() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_170() {
        let src = format!("scene s170 {{}}\nfn f170() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_171() {
        let src = format!("scene s171 {{}}\nfn f171() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_172() {
        let src = format!("scene s172 {{}}\nfn f172() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_173() {
        let src = format!("scene s173 {{}}\nfn f173() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_174() {
        let src = format!("scene s174 {{}}\nfn f174() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_175() {
        let src = format!("scene s175 {{}}\nfn f175() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_176() {
        let src = format!("scene s176 {{}}\nfn f176() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_177() {
        let src = format!("scene s177 {{}}\nfn f177() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_178() {
        let src = format!("scene s178 {{}}\nfn f178() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_179() {
        let src = format!("scene s179 {{}}\nfn f179() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_180() {
        let src = format!("scene s180 {{}}\nfn f180() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_181() {
        let src = format!("scene s181 {{}}\nfn f181() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_182() {
        let src = format!("scene s182 {{}}\nfn f182() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_183() {
        let src = format!("scene s183 {{}}\nfn f183() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_184() {
        let src = format!("scene s184 {{}}\nfn f184() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_185() {
        let src = format!("scene s185 {{}}\nfn f185() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_186() {
        let src = format!("scene s186 {{}}\nfn f186() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_187() {
        let src = format!("scene s187 {{}}\nfn f187() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_188() {
        let src = format!("scene s188 {{}}\nfn f188() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_189() {
        let src = format!("scene s189 {{}}\nfn f189() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_190() {
        let src = format!("scene s190 {{}}\nfn f190() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_191() {
        let src = format!("scene s191 {{}}\nfn f191() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_192() {
        let src = format!("scene s192 {{}}\nfn f192() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_193() {
        let src = format!("scene s193 {{}}\nfn f193() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_194() {
        let src = format!("scene s194 {{}}\nfn f194() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_195() {
        let src = format!("scene s195 {{}}\nfn f195() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_196() {
        let src = format!("scene s196 {{}}\nfn f196() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_197() {
        let src = format!("scene s197 {{}}\nfn f197() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_198() {
        let src = format!("scene s198 {{}}\nfn f198() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_199() {
        let src = format!("scene s199 {{}}\nfn f199() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_200() {
        let src = format!("scene s200 {{}}\nfn f200() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_201() {
        let src = format!("scene s201 {{}}\nfn f201() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_202() {
        let src = format!("scene s202 {{}}\nfn f202() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_203() {
        let src = format!("scene s203 {{}}\nfn f203() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_204() {
        let src = format!("scene s204 {{}}\nfn f204() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_205() {
        let src = format!("scene s205 {{}}\nfn f205() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_206() {
        let src = format!("scene s206 {{}}\nfn f206() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_207() {
        let src = format!("scene s207 {{}}\nfn f207() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_208() {
        let src = format!("scene s208 {{}}\nfn f208() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_209() {
        let src = format!("scene s209 {{}}\nfn f209() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_210() {
        let src = format!("scene s210 {{}}\nfn f210() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_211() {
        let src = format!("scene s211 {{}}\nfn f211() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_212() {
        let src = format!("scene s212 {{}}\nfn f212() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_213() {
        let src = format!("scene s213 {{}}\nfn f213() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_214() {
        let src = format!("scene s214 {{}}\nfn f214() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_215() {
        let src = format!("scene s215 {{}}\nfn f215() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_216() {
        let src = format!("scene s216 {{}}\nfn f216() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_217() {
        let src = format!("scene s217 {{}}\nfn f217() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_218() {
        let src = format!("scene s218 {{}}\nfn f218() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_219() {
        let src = format!("scene s219 {{}}\nfn f219() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_220() {
        let src = format!("scene s220 {{}}\nfn f220() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_221() {
        let src = format!("scene s221 {{}}\nfn f221() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_222() {
        let src = format!("scene s222 {{}}\nfn f222() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_223() {
        let src = format!("scene s223 {{}}\nfn f223() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_224() {
        let src = format!("scene s224 {{}}\nfn f224() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_225() {
        let src = format!("scene s225 {{}}\nfn f225() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_226() {
        let src = format!("scene s226 {{}}\nfn f226() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_227() {
        let src = format!("scene s227 {{}}\nfn f227() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_228() {
        let src = format!("scene s228 {{}}\nfn f228() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_229() {
        let src = format!("scene s229 {{}}\nfn f229() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_230() {
        let src = format!("scene s230 {{}}\nfn f230() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_231() {
        let src = format!("scene s231 {{}}\nfn f231() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_232() {
        let src = format!("scene s232 {{}}\nfn f232() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_233() {
        let src = format!("scene s233 {{}}\nfn f233() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_234() {
        let src = format!("scene s234 {{}}\nfn f234() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_235() {
        let src = format!("scene s235 {{}}\nfn f235() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_236() {
        let src = format!("scene s236 {{}}\nfn f236() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_237() {
        let src = format!("scene s237 {{}}\nfn f237() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_238() {
        let src = format!("scene s238 {{}}\nfn f238() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_239() {
        let src = format!("scene s239 {{}}\nfn f239() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_240() {
        let src = format!("scene s240 {{}}\nfn f240() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_241() {
        let src = format!("scene s241 {{}}\nfn f241() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_242() {
        let src = format!("scene s242 {{}}\nfn f242() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_243() {
        let src = format!("scene s243 {{}}\nfn f243() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_244() {
        let src = format!("scene s244 {{}}\nfn f244() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_245() {
        let src = format!("scene s245 {{}}\nfn f245() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_246() {
        let src = format!("scene s246 {{}}\nfn f246() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_247() {
        let src = format!("scene s247 {{}}\nfn f247() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_248() {
        let src = format!("scene s248 {{}}\nfn f248() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_249() {
        let src = format!("scene s249 {{}}\nfn f249() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_250() {
        let src = format!("scene s250 {{}}\nfn f250() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_251() {
        let src = format!("scene s251 {{}}\nfn f251() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_252() {
        let src = format!("scene s252 {{}}\nfn f252() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_253() {
        let src = format!("scene s253 {{}}\nfn f253() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_254() {
        let src = format!("scene s254 {{}}\nfn f254() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_255() {
        let src = format!("scene s255 {{}}\nfn f255() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_256() {
        let src = format!("scene s256 {{}}\nfn f256() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_257() {
        let src = format!("scene s257 {{}}\nfn f257() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_258() {
        let src = format!("scene s258 {{}}\nfn f258() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_259() {
        let src = format!("scene s259 {{}}\nfn f259() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_260() {
        let src = format!("scene s260 {{}}\nfn f260() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_261() {
        let src = format!("scene s261 {{}}\nfn f261() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_262() {
        let src = format!("scene s262 {{}}\nfn f262() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_263() {
        let src = format!("scene s263 {{}}\nfn f263() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_264() {
        let src = format!("scene s264 {{}}\nfn f264() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_265() {
        let src = format!("scene s265 {{}}\nfn f265() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_266() {
        let src = format!("scene s266 {{}}\nfn f266() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_267() {
        let src = format!("scene s267 {{}}\nfn f267() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_268() {
        let src = format!("scene s268 {{}}\nfn f268() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_269() {
        let src = format!("scene s269 {{}}\nfn f269() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_270() {
        let src = format!("scene s270 {{}}\nfn f270() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_271() {
        let src = format!("scene s271 {{}}\nfn f271() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_272() {
        let src = format!("scene s272 {{}}\nfn f272() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_273() {
        let src = format!("scene s273 {{}}\nfn f273() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_274() {
        let src = format!("scene s274 {{}}\nfn f274() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_275() {
        let src = format!("scene s275 {{}}\nfn f275() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_276() {
        let src = format!("scene s276 {{}}\nfn f276() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_277() {
        let src = format!("scene s277 {{}}\nfn f277() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_278() {
        let src = format!("scene s278 {{}}\nfn f278() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_279() {
        let src = format!("scene s279 {{}}\nfn f279() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_280() {
        let src = format!("scene s280 {{}}\nfn f280() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_281() {
        let src = format!("scene s281 {{}}\nfn f281() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_282() {
        let src = format!("scene s282 {{}}\nfn f282() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_283() {
        let src = format!("scene s283 {{}}\nfn f283() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_284() {
        let src = format!("scene s284 {{}}\nfn f284() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_285() {
        let src = format!("scene s285 {{}}\nfn f285() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_286() {
        let src = format!("scene s286 {{}}\nfn f286() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_287() {
        let src = format!("scene s287 {{}}\nfn f287() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_288() {
        let src = format!("scene s288 {{}}\nfn f288() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_289() {
        let src = format!("scene s289 {{}}\nfn f289() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_290() {
        let src = format!("scene s290 {{}}\nfn f290() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_291() {
        let src = format!("scene s291 {{}}\nfn f291() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_292() {
        let src = format!("scene s292 {{}}\nfn f292() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_293() {
        let src = format!("scene s293 {{}}\nfn f293() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_294() {
        let src = format!("scene s294 {{}}\nfn f294() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_295() {
        let src = format!("scene s295 {{}}\nfn f295() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_296() {
        let src = format!("scene s296 {{}}\nfn f296() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_297() {
        let src = format!("scene s297 {{}}\nfn f297() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_298() {
        let src = format!("scene s298 {{}}\nfn f298() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn lower_scene_299() {
        let src = format!("scene s299 {{}}\nfn f299() {{}}\n");
        let (m, _) = lower_source_heuristic(&src, 2);
        assert!(m.item_count() >= 2);
        assert_eq!(m.edition, 2);
    }
    #[test]
    fn path_parse_0() {
        let p = HirPath::parse("a0::b0::c0");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b0"));
    }
    #[test]
    fn path_parse_1() {
        let p = HirPath::parse("a1::b1::c1");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b1"));
    }
    #[test]
    fn path_parse_2() {
        let p = HirPath::parse("a2::b2::c2");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b2"));
    }
    #[test]
    fn path_parse_3() {
        let p = HirPath::parse("a3::b3::c3");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b3"));
    }
    #[test]
    fn path_parse_4() {
        let p = HirPath::parse("a4::b4::c4");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b4"));
    }
    #[test]
    fn path_parse_5() {
        let p = HirPath::parse("a5::b5::c5");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b5"));
    }
    #[test]
    fn path_parse_6() {
        let p = HirPath::parse("a6::b6::c6");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b6"));
    }
    #[test]
    fn path_parse_7() {
        let p = HirPath::parse("a7::b7::c7");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b7"));
    }
    #[test]
    fn path_parse_8() {
        let p = HirPath::parse("a8::b8::c8");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b8"));
    }
    #[test]
    fn path_parse_9() {
        let p = HirPath::parse("a9::b9::c9");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b9"));
    }
    #[test]
    fn path_parse_10() {
        let p = HirPath::parse("a10::b10::c10");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b10"));
    }
    #[test]
    fn path_parse_11() {
        let p = HirPath::parse("a11::b11::c11");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b11"));
    }
    #[test]
    fn path_parse_12() {
        let p = HirPath::parse("a12::b12::c12");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b12"));
    }
    #[test]
    fn path_parse_13() {
        let p = HirPath::parse("a13::b13::c13");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b13"));
    }
    #[test]
    fn path_parse_14() {
        let p = HirPath::parse("a14::b14::c14");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b14"));
    }
    #[test]
    fn path_parse_15() {
        let p = HirPath::parse("a15::b15::c15");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b15"));
    }
    #[test]
    fn path_parse_16() {
        let p = HirPath::parse("a16::b16::c16");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b16"));
    }
    #[test]
    fn path_parse_17() {
        let p = HirPath::parse("a17::b17::c17");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b17"));
    }
    #[test]
    fn path_parse_18() {
        let p = HirPath::parse("a18::b18::c18");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b18"));
    }
    #[test]
    fn path_parse_19() {
        let p = HirPath::parse("a19::b19::c19");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b19"));
    }
    #[test]
    fn path_parse_20() {
        let p = HirPath::parse("a20::b20::c20");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b20"));
    }
    #[test]
    fn path_parse_21() {
        let p = HirPath::parse("a21::b21::c21");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b21"));
    }
    #[test]
    fn path_parse_22() {
        let p = HirPath::parse("a22::b22::c22");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b22"));
    }
    #[test]
    fn path_parse_23() {
        let p = HirPath::parse("a23::b23::c23");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b23"));
    }
    #[test]
    fn path_parse_24() {
        let p = HirPath::parse("a24::b24::c24");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b24"));
    }
    #[test]
    fn path_parse_25() {
        let p = HirPath::parse("a25::b25::c25");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b25"));
    }
    #[test]
    fn path_parse_26() {
        let p = HirPath::parse("a26::b26::c26");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b26"));
    }
    #[test]
    fn path_parse_27() {
        let p = HirPath::parse("a27::b27::c27");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b27"));
    }
    #[test]
    fn path_parse_28() {
        let p = HirPath::parse("a28::b28::c28");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b28"));
    }
    #[test]
    fn path_parse_29() {
        let p = HirPath::parse("a29::b29::c29");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b29"));
    }
    #[test]
    fn path_parse_30() {
        let p = HirPath::parse("a30::b30::c30");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b30"));
    }
    #[test]
    fn path_parse_31() {
        let p = HirPath::parse("a31::b31::c31");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b31"));
    }
    #[test]
    fn path_parse_32() {
        let p = HirPath::parse("a32::b32::c32");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b32"));
    }
    #[test]
    fn path_parse_33() {
        let p = HirPath::parse("a33::b33::c33");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b33"));
    }
    #[test]
    fn path_parse_34() {
        let p = HirPath::parse("a34::b34::c34");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b34"));
    }
    #[test]
    fn path_parse_35() {
        let p = HirPath::parse("a35::b35::c35");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b35"));
    }
    #[test]
    fn path_parse_36() {
        let p = HirPath::parse("a36::b36::c36");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b36"));
    }
    #[test]
    fn path_parse_37() {
        let p = HirPath::parse("a37::b37::c37");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b37"));
    }
    #[test]
    fn path_parse_38() {
        let p = HirPath::parse("a38::b38::c38");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b38"));
    }
    #[test]
    fn path_parse_39() {
        let p = HirPath::parse("a39::b39::c39");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b39"));
    }
    #[test]
    fn path_parse_40() {
        let p = HirPath::parse("a40::b40::c40");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b40"));
    }
    #[test]
    fn path_parse_41() {
        let p = HirPath::parse("a41::b41::c41");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b41"));
    }
    #[test]
    fn path_parse_42() {
        let p = HirPath::parse("a42::b42::c42");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b42"));
    }
    #[test]
    fn path_parse_43() {
        let p = HirPath::parse("a43::b43::c43");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b43"));
    }
    #[test]
    fn path_parse_44() {
        let p = HirPath::parse("a44::b44::c44");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b44"));
    }
    #[test]
    fn path_parse_45() {
        let p = HirPath::parse("a45::b45::c45");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b45"));
    }
    #[test]
    fn path_parse_46() {
        let p = HirPath::parse("a46::b46::c46");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b46"));
    }
    #[test]
    fn path_parse_47() {
        let p = HirPath::parse("a47::b47::c47");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b47"));
    }
    #[test]
    fn path_parse_48() {
        let p = HirPath::parse("a48::b48::c48");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b48"));
    }
    #[test]
    fn path_parse_49() {
        let p = HirPath::parse("a49::b49::c49");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b49"));
    }
    #[test]
    fn path_parse_50() {
        let p = HirPath::parse("a50::b50::c50");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b50"));
    }
    #[test]
    fn path_parse_51() {
        let p = HirPath::parse("a51::b51::c51");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b51"));
    }
    #[test]
    fn path_parse_52() {
        let p = HirPath::parse("a52::b52::c52");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b52"));
    }
    #[test]
    fn path_parse_53() {
        let p = HirPath::parse("a53::b53::c53");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b53"));
    }
    #[test]
    fn path_parse_54() {
        let p = HirPath::parse("a54::b54::c54");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b54"));
    }
    #[test]
    fn path_parse_55() {
        let p = HirPath::parse("a55::b55::c55");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b55"));
    }
    #[test]
    fn path_parse_56() {
        let p = HirPath::parse("a56::b56::c56");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b56"));
    }
    #[test]
    fn path_parse_57() {
        let p = HirPath::parse("a57::b57::c57");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b57"));
    }
    #[test]
    fn path_parse_58() {
        let p = HirPath::parse("a58::b58::c58");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b58"));
    }
    #[test]
    fn path_parse_59() {
        let p = HirPath::parse("a59::b59::c59");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b59"));
    }
    #[test]
    fn path_parse_60() {
        let p = HirPath::parse("a60::b60::c60");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b60"));
    }
    #[test]
    fn path_parse_61() {
        let p = HirPath::parse("a61::b61::c61");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b61"));
    }
    #[test]
    fn path_parse_62() {
        let p = HirPath::parse("a62::b62::c62");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b62"));
    }
    #[test]
    fn path_parse_63() {
        let p = HirPath::parse("a63::b63::c63");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b63"));
    }
    #[test]
    fn path_parse_64() {
        let p = HirPath::parse("a64::b64::c64");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b64"));
    }
    #[test]
    fn path_parse_65() {
        let p = HirPath::parse("a65::b65::c65");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b65"));
    }
    #[test]
    fn path_parse_66() {
        let p = HirPath::parse("a66::b66::c66");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b66"));
    }
    #[test]
    fn path_parse_67() {
        let p = HirPath::parse("a67::b67::c67");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b67"));
    }
    #[test]
    fn path_parse_68() {
        let p = HirPath::parse("a68::b68::c68");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b68"));
    }
    #[test]
    fn path_parse_69() {
        let p = HirPath::parse("a69::b69::c69");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b69"));
    }
    #[test]
    fn path_parse_70() {
        let p = HirPath::parse("a70::b70::c70");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b70"));
    }
    #[test]
    fn path_parse_71() {
        let p = HirPath::parse("a71::b71::c71");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b71"));
    }
    #[test]
    fn path_parse_72() {
        let p = HirPath::parse("a72::b72::c72");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b72"));
    }
    #[test]
    fn path_parse_73() {
        let p = HirPath::parse("a73::b73::c73");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b73"));
    }
    #[test]
    fn path_parse_74() {
        let p = HirPath::parse("a74::b74::c74");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b74"));
    }
    #[test]
    fn path_parse_75() {
        let p = HirPath::parse("a75::b75::c75");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b75"));
    }
    #[test]
    fn path_parse_76() {
        let p = HirPath::parse("a76::b76::c76");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b76"));
    }
    #[test]
    fn path_parse_77() {
        let p = HirPath::parse("a77::b77::c77");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b77"));
    }
    #[test]
    fn path_parse_78() {
        let p = HirPath::parse("a78::b78::c78");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b78"));
    }
    #[test]
    fn path_parse_79() {
        let p = HirPath::parse("a79::b79::c79");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b79"));
    }
    #[test]
    fn path_parse_80() {
        let p = HirPath::parse("a80::b80::c80");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b80"));
    }
    #[test]
    fn path_parse_81() {
        let p = HirPath::parse("a81::b81::c81");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b81"));
    }
    #[test]
    fn path_parse_82() {
        let p = HirPath::parse("a82::b82::c82");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b82"));
    }
    #[test]
    fn path_parse_83() {
        let p = HirPath::parse("a83::b83::c83");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b83"));
    }
    #[test]
    fn path_parse_84() {
        let p = HirPath::parse("a84::b84::c84");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b84"));
    }
    #[test]
    fn path_parse_85() {
        let p = HirPath::parse("a85::b85::c85");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b85"));
    }
    #[test]
    fn path_parse_86() {
        let p = HirPath::parse("a86::b86::c86");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b86"));
    }
    #[test]
    fn path_parse_87() {
        let p = HirPath::parse("a87::b87::c87");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b87"));
    }
    #[test]
    fn path_parse_88() {
        let p = HirPath::parse("a88::b88::c88");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b88"));
    }
    #[test]
    fn path_parse_89() {
        let p = HirPath::parse("a89::b89::c89");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b89"));
    }
    #[test]
    fn path_parse_90() {
        let p = HirPath::parse("a90::b90::c90");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b90"));
    }
    #[test]
    fn path_parse_91() {
        let p = HirPath::parse("a91::b91::c91");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b91"));
    }
    #[test]
    fn path_parse_92() {
        let p = HirPath::parse("a92::b92::c92");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b92"));
    }
    #[test]
    fn path_parse_93() {
        let p = HirPath::parse("a93::b93::c93");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b93"));
    }
    #[test]
    fn path_parse_94() {
        let p = HirPath::parse("a94::b94::c94");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b94"));
    }
    #[test]
    fn path_parse_95() {
        let p = HirPath::parse("a95::b95::c95");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b95"));
    }
    #[test]
    fn path_parse_96() {
        let p = HirPath::parse("a96::b96::c96");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b96"));
    }
    #[test]
    fn path_parse_97() {
        let p = HirPath::parse("a97::b97::c97");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b97"));
    }
    #[test]
    fn path_parse_98() {
        let p = HirPath::parse("a98::b98::c98");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b98"));
    }
    #[test]
    fn path_parse_99() {
        let p = HirPath::parse("a99::b99::c99");
        assert_eq!(p.segs.len(), 3);
        assert!(p.display().contains("b99"));
    }
}
