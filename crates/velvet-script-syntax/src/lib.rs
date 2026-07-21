//! Velvet Script 2 syntax tables: keywords, operators, editions, diagnostics.

#![deny(missing_docs)]

/// Language edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    /// Original surface.
    V1 = 1,
    /// Typed rust-like surface.
    V2 = 2,
}
impl Edition {
    /// Parse.
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            _ => None,
        }
    }
    /// Latest.
    pub fn latest() -> Self {
        Self::V2
    }
    /// As number.
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// `as`
    AsKw,
    /// `async`
    AsyncKw,
    /// `await`
    AwaitKw,
    /// `break`
    BreakKw,
    /// `call`
    CallKw,
    /// `character`
    CharacterKw,
    /// `const`
    ConstKw,
    /// `continue`
    ContinueKw,
    /// `crate`
    CrateKw,
    /// `else`
    ElseKw,
    /// `enum`
    EnumKw,
    /// `extern`
    ExternKw,
    /// `false`
    FalseKw,
    /// `fn`
    FnKw,
    /// `for`
    ForKw,
    /// `function`
    FunctionKw,
    /// `if`
    IfKw,
    /// `impl`
    ImplKw,
    /// `import`
    ImportKw,
    /// `in`
    InKw,
    /// `jump`
    JumpKw,
    /// `let`
    LetKw,
    /// `loop`
    LoopKw,
    /// `match`
    MatchKw,
    /// `menu`
    MenuKw,
    /// `mod`
    ModKw,
    /// `move`
    MoveKw,
    /// `mut`
    MutKw,
    /// `pub`
    PubKw,
    /// `ref`
    RefKw,
    /// `return`
    ReturnKw,
    /// `scene`
    SceneKw,
    /// `screen`
    ScreenKw,
    /// `self`
    SelfValue,
    /// `Self`
    SelfType,
    /// `show`
    ShowKw,
    /// `hide`
    HideKw,
    /// `say`
    SayKw,
    /// `state`
    StateKw,
    /// `static`
    StaticKw,
    /// `struct`
    StructKw,
    /// `super`
    SuperKw,
    /// `trait`
    TraitKw,
    /// `true`
    TrueKw,
    /// `type`
    TypeKw,
    /// `use`
    UseKw,
    /// `where`
    WhereKw,
    /// `while`
    WhileKw,
    /// `with`
    WithKw,
    /// `transform`
    TransformKw,
    /// `layer`
    LayerKw,
    /// `background`
    BackgroundKw,
    /// `music`
    MusicKw,
    /// `choice`
    ChoiceKw,
    /// `option`
    OptionKw,
    /// `Ok`
    OkKw,
    /// `Err`
    ErrKw,
    /// `Some`
    SomeKw,
    /// `None`
    NoneKw,
    /// `Result`
    ResultKw,
    /// `Option`
    OptionTypeKw,
    /// `try`
    TryKw,
}

impl Keyword {
    /// Text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsKw => "as",
            Self::AsyncKw => "async",
            Self::AwaitKw => "await",
            Self::BreakKw => "break",
            Self::CallKw => "call",
            Self::CharacterKw => "character",
            Self::ConstKw => "const",
            Self::ContinueKw => "continue",
            Self::CrateKw => "crate",
            Self::ElseKw => "else",
            Self::EnumKw => "enum",
            Self::ExternKw => "extern",
            Self::FalseKw => "false",
            Self::FnKw => "fn",
            Self::ForKw => "for",
            Self::FunctionKw => "function",
            Self::IfKw => "if",
            Self::ImplKw => "impl",
            Self::ImportKw => "import",
            Self::InKw => "in",
            Self::JumpKw => "jump",
            Self::LetKw => "let",
            Self::LoopKw => "loop",
            Self::MatchKw => "match",
            Self::MenuKw => "menu",
            Self::ModKw => "mod",
            Self::MoveKw => "move",
            Self::MutKw => "mut",
            Self::PubKw => "pub",
            Self::RefKw => "ref",
            Self::ReturnKw => "return",
            Self::SceneKw => "scene",
            Self::ScreenKw => "screen",
            Self::SelfValue => "self",
            Self::SelfType => "Self",
            Self::ShowKw => "show",
            Self::HideKw => "hide",
            Self::SayKw => "say",
            Self::StateKw => "state",
            Self::StaticKw => "static",
            Self::StructKw => "struct",
            Self::SuperKw => "super",
            Self::TraitKw => "trait",
            Self::TrueKw => "true",
            Self::TypeKw => "type",
            Self::UseKw => "use",
            Self::WhereKw => "where",
            Self::WhileKw => "while",
            Self::WithKw => "with",
            Self::TransformKw => "transform",
            Self::LayerKw => "layer",
            Self::BackgroundKw => "background",
            Self::MusicKw => "music",
            Self::ChoiceKw => "choice",
            Self::OptionKw => "option",
            Self::OkKw => "Ok",
            Self::ErrKw => "Err",
            Self::SomeKw => "Some",
            Self::NoneKw => "None",
            Self::ResultKw => "Result",
            Self::OptionTypeKw => "Option",
            Self::TryKw => "try",
        }
    }
    /// Lookup.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "as" => Some(Self::AsKw),
            "async" => Some(Self::AsyncKw),
            "await" => Some(Self::AwaitKw),
            "break" => Some(Self::BreakKw),
            "call" => Some(Self::CallKw),
            "character" => Some(Self::CharacterKw),
            "const" => Some(Self::ConstKw),
            "continue" => Some(Self::ContinueKw),
            "crate" => Some(Self::CrateKw),
            "else" => Some(Self::ElseKw),
            "enum" => Some(Self::EnumKw),
            "extern" => Some(Self::ExternKw),
            "false" => Some(Self::FalseKw),
            "fn" => Some(Self::FnKw),
            "for" => Some(Self::ForKw),
            "function" => Some(Self::FunctionKw),
            "if" => Some(Self::IfKw),
            "impl" => Some(Self::ImplKw),
            "import" => Some(Self::ImportKw),
            "in" => Some(Self::InKw),
            "jump" => Some(Self::JumpKw),
            "let" => Some(Self::LetKw),
            "loop" => Some(Self::LoopKw),
            "match" => Some(Self::MatchKw),
            "menu" => Some(Self::MenuKw),
            "mod" => Some(Self::ModKw),
            "move" => Some(Self::MoveKw),
            "mut" => Some(Self::MutKw),
            "pub" => Some(Self::PubKw),
            "ref" => Some(Self::RefKw),
            "return" => Some(Self::ReturnKw),
            "scene" => Some(Self::SceneKw),
            "screen" => Some(Self::ScreenKw),
            "self" => Some(Self::SelfValue),
            "Self" => Some(Self::SelfType),
            "show" => Some(Self::ShowKw),
            "hide" => Some(Self::HideKw),
            "say" => Some(Self::SayKw),
            "state" => Some(Self::StateKw),
            "static" => Some(Self::StaticKw),
            "struct" => Some(Self::StructKw),
            "super" => Some(Self::SuperKw),
            "trait" => Some(Self::TraitKw),
            "true" => Some(Self::TrueKw),
            "type" => Some(Self::TypeKw),
            "use" => Some(Self::UseKw),
            "where" => Some(Self::WhereKw),
            "while" => Some(Self::WhileKw),
            "with" => Some(Self::WithKw),
            "transform" => Some(Self::TransformKw),
            "layer" => Some(Self::LayerKw),
            "background" => Some(Self::BackgroundKw),
            "music" => Some(Self::MusicKw),
            "choice" => Some(Self::ChoiceKw),
            "option" => Some(Self::OptionKw),
            "Ok" => Some(Self::OkKw),
            "Err" => Some(Self::ErrKw),
            "Some" => Some(Self::SomeKw),
            "None" => Some(Self::NoneKw),
            "Result" => Some(Self::ResultKw),
            "Option" => Some(Self::OptionTypeKw),
            "try" => Some(Self::TryKw),
            _ => None,
        }
    }
    /// All.
    pub fn all() -> &'static [Self] {
        &[
            Self::AsKw,
            Self::AsyncKw,
            Self::AwaitKw,
            Self::BreakKw,
            Self::CallKw,
            Self::CharacterKw,
            Self::ConstKw,
            Self::ContinueKw,
            Self::CrateKw,
            Self::ElseKw,
            Self::EnumKw,
            Self::ExternKw,
            Self::FalseKw,
            Self::FnKw,
            Self::ForKw,
            Self::FunctionKw,
            Self::IfKw,
            Self::ImplKw,
            Self::ImportKw,
            Self::InKw,
            Self::JumpKw,
            Self::LetKw,
            Self::LoopKw,
            Self::MatchKw,
            Self::MenuKw,
            Self::ModKw,
            Self::MoveKw,
            Self::MutKw,
            Self::PubKw,
            Self::RefKw,
            Self::ReturnKw,
            Self::SceneKw,
            Self::ScreenKw,
            Self::SelfValue,
            Self::SelfType,
            Self::ShowKw,
            Self::HideKw,
            Self::SayKw,
            Self::StateKw,
            Self::StaticKw,
            Self::StructKw,
            Self::SuperKw,
            Self::TraitKw,
            Self::TrueKw,
            Self::TypeKw,
            Self::UseKw,
            Self::WhereKw,
            Self::WhileKw,
            Self::WithKw,
            Self::TransformKw,
            Self::LayerKw,
            Self::BackgroundKw,
            Self::MusicKw,
            Self::ChoiceKw,
            Self::OptionKw,
            Self::OkKw,
            Self::ErrKw,
            Self::SomeKw,
            Self::NoneKw,
            Self::ResultKw,
            Self::OptionTypeKw,
            Self::TryKw,
        ]
    }
}

/// Operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Rem,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`
    Shl,
    /// `>>`
    Shr,
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
}
impl Op {
    /// Symbol.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
    /// Precedence (higher binds tighter).
    pub fn precedence(self) -> u8 {
        match self {
            Self::Add => 40,
            Self::Sub => 40,
            Self::Mul => 50,
            Self::Div => 50,
            Self::Rem => 50,
            Self::Eq => 18,
            Self::Ne => 18,
            Self::Lt => 20,
            Self::Le => 20,
            Self::Gt => 20,
            Self::Ge => 20,
            Self::And => 15,
            Self::Or => 12,
            Self::BitAnd => 30,
            Self::BitOr => 26,
            Self::BitXor => 28,
            Self::Shl => 35,
            Self::Shr => 35,
            Self::Assign => 5,
            Self::AddAssign => 5,
            Self::SubAssign => 5,
            Self::MulAssign => 5,
            Self::DivAssign => 5,
        }
    }
}

/// Stable diagnostic codes with real messages (not E0001..E0500 padding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DiagCode {
    /// Unexpected token.
    UnexpectedToken = 1,
    /// Unterminated string.
    UnterminatedString = 2,
    /// Invalid number literal.
    InvalidNumber = 3,
    /// Unknown keyword / reserved misuse.
    UnknownKeyword = 4,
    /// Unmatched brace / paren / bracket.
    UnmatchedDelimiter = 5,
    /// Expected expression.
    ExpectedExpr = 6,
    /// Expected type.
    ExpectedType = 7,
    /// Duplicate definition.
    DuplicateDefinition = 8,
    /// Unresolved name.
    UnresolvedName = 9,
    /// Type mismatch.
    TypeMismatch = 10,
    /// Invalid jump / scene target.
    InvalidJumpTarget = 11,
    /// Missing `main` or entry.
    MissingEntry = 12,
    /// Feature not yet lowered (struct/enum/field/mod…).
    UnsupportedHir = 13,
    /// Internal compiler bug placeholder.
    Internal = 99,
}

impl DiagCode {
    /// Numeric code.
    pub fn code(self) -> u16 {
        self as u16
    }

    /// Short label `VS####`.
    pub fn label(self) -> String {
        format!("VS{:04}", self.code() as u32)
    }

    /// Human-readable message (not just the label).
    pub fn message(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected token",
            Self::UnterminatedString => "unterminated string literal",
            Self::InvalidNumber => "invalid number literal",
            Self::UnknownKeyword => "unknown or misplaced keyword",
            Self::UnmatchedDelimiter => "unmatched delimiter",
            Self::ExpectedExpr => "expected expression",
            Self::ExpectedType => "expected type",
            Self::DuplicateDefinition => "duplicate definition",
            Self::UnresolvedName => "unresolved name",
            Self::TypeMismatch => "type mismatch",
            Self::InvalidJumpTarget => "invalid jump or scene target",
            Self::MissingEntry => "missing entry point",
            Self::UnsupportedHir => "construct not yet supported in lowering",
            Self::Internal => "internal compiler error",
        }
    }

    /// All real codes (dense catalog, not 500 placeholders).
    pub fn all() -> &'static [Self] {
        &[
            Self::UnexpectedToken,
            Self::UnterminatedString,
            Self::InvalidNumber,
            Self::UnknownKeyword,
            Self::UnmatchedDelimiter,
            Self::ExpectedExpr,
            Self::ExpectedType,
            Self::DuplicateDefinition,
            Self::UnresolvedName,
            Self::TypeMismatch,
            Self::InvalidJumpTarget,
            Self::MissingEntry,
            Self::UnsupportedHir,
            Self::Internal,
        ]
    }

    /// Lookup by numeric code.
    pub fn from_code(n: u16) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.code() == n)
    }
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_roundtrip() {
        for k in Keyword::all() {
            assert_eq!(Keyword::from_str(k.as_str()), Some(*k));
        }
        assert!(Keyword::all().len() > 20);
        assert!(Keyword::all().len() < 120);
    }

    #[test]
    fn edition_v2() {
        assert_eq!(Edition::from_u32(2), Some(Edition::V2));
        assert_eq!(Edition::latest(), Edition::V2);
    }

    #[test]
    fn diag_catalog_small_and_messaged() {
        assert!(DiagCode::all().len() < 40);
        assert!(DiagCode::all().len() >= 10);
        for d in DiagCode::all() {
            assert!(!d.message().is_empty());
            assert!(d.label().starts_with("VS"));
            assert_eq!(DiagCode::from_code(d.code()), Some(*d));
        }
        assert_eq!(DiagCode::TypeMismatch.message(), "type mismatch");
        assert_eq!(DiagCode::UnsupportedHir.code(), 13);
        // no 500-wide fake range
        assert!(DiagCode::from_code(500).is_none());
        assert!(DiagCode::from_code(1).is_some());
    }

    #[test]
    fn op_precedence() {
        assert!(Op::Mul.precedence() > Op::Add.precedence());
        assert!(Op::Add.precedence() > Op::Eq.precedence());
        assert_eq!(Op::Assign.as_str(), "=");
    }
}
