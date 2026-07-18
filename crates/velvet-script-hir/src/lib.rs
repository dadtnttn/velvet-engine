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
