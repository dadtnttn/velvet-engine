//! Tokens for Velvet Story (indent-aware).

use crate::span::Span;

/// Token kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Identifier / bareword.
    Ident(String),
    /// Integer literal.
    Int(i64),
    /// Float literal.
    Float(String),
    /// String literal (content, unescaped).
    String(String),
    /// `:`
    Colon,
    /// `@`
    At,
    /// `=`
    Eq,
    /// `==`
    EqEq,
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
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `and`
    And,
    /// `or`
    Or,
    /// `not`
    Not,
    /// `true`
    True,
    /// `false`
    False,
    /// Indent (spaces count).
    Indent(usize),
    /// Dedent.
    Dedent,
    /// Newline.
    Newline,
    /// End of file.
    Eof,
    /// Comment text (kept for format).
    Comment(String),
}

/// Token with span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Kind.
    pub kind: TokenKind,
    /// Span.
    pub span: Span,
}

impl Token {
    /// New.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
