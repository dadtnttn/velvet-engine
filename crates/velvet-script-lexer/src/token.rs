//! Token kinds and values.

use logos::Logos;
use thiserror::Error;

/// Byte offset span `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
}

impl Span {
    /// Length in bytes.
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Empty.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

/// Lexer errors with location.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LexerError {
    /// Unexpected character sequence.
    #[error("unexpected input at {line}:{column}: '{snippet}'")]
    Unexpected {
        /// Line 1-based.
        line: u32,
        /// Column 1-based.
        column: u32,
        /// Snippet.
        snippet: String,
    },
    /// Bad string escape / unterminated.
    #[error("invalid string at {line}:{column}: {message}")]
    InvalidString {
        /// Line.
        line: u32,
        /// Column.
        column: u32,
        /// Message.
        message: String,
    },
}

/// Semantic token after processing logos kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Identifier or keyword word.
    Ident(String),
    /// Integer literal.
    Int(i64),
    /// Float literal.
    Float(f64),
    /// String literal (unescaped content).
    String(String),
    /// `true`
    True,
    /// `false`
    False,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `:`
    Colon,
    /// `;`
    Semi,
    /// `=`
    Assign,
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
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `+=`
    PlusAssign,
    /// `-=`
    MinusAssign,
    /// `*=`
    StarAssign,
    /// `/=`
    SlashAssign,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
    /// `->`
    Arrow,
}

/// Logos-driven raw kinds.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[allow(missing_docs)]
pub enum TokenKind {
    /// Line comment.
    #[regex(r"//[^\n]*", logos::skip)]
    Comment,

    /// Identifier.
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,
    /// Float (must be before int).
    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?")]
    Float,
    /// Integer.
    #[regex(r"[0-9]+")]
    Int,
    /// String double-quoted.
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,

    /// Symbols
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("==")]
    Eq,
    #[token("!=")]
    Ne,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("->")]
    Arrow,
    #[token("=")]
    Assign,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("!")]
    Bang,
}

impl TokenKind {
    /// Convert logos kind + slice into a [`Token`].
    pub fn into_token(self, slice: &str) -> Result<Token, LexerError> {
        Ok(match self {
            TokenKind::Comment => {
                unreachable!("filtered before into_token")
            }
            TokenKind::Ident => match slice {
                "true" => Token::True,
                "false" => Token::False,
                other => Token::Ident(other.to_string()),
            },
            TokenKind::Int => Token::Int(slice.parse().map_err(|_| LexerError::Unexpected {
                line: 0,
                column: 0,
                snippet: slice.into(),
            })?),
            TokenKind::Float => {
                Token::Float(slice.parse().map_err(|_| LexerError::Unexpected {
                    line: 0,
                    column: 0,
                    snippet: slice.into(),
                })?)
            }
            TokenKind::String => Token::String(unescape_string(slice)?),
            TokenKind::LBrace => Token::LBrace,
            TokenKind::RBrace => Token::RBrace,
            TokenKind::LParen => Token::LParen,
            TokenKind::RParen => Token::RParen,
            TokenKind::LBracket => Token::LBracket,
            TokenKind::RBracket => Token::RBracket,
            TokenKind::Comma => Token::Comma,
            TokenKind::Dot => Token::Dot,
            TokenKind::Colon => Token::Colon,
            TokenKind::Semi => Token::Semi,
            TokenKind::Eq => Token::Eq,
            TokenKind::Ne => Token::Ne,
            TokenKind::Le => Token::Le,
            TokenKind::Ge => Token::Ge,
            TokenKind::PlusAssign => Token::PlusAssign,
            TokenKind::MinusAssign => Token::MinusAssign,
            TokenKind::StarAssign => Token::StarAssign,
            TokenKind::SlashAssign => Token::SlashAssign,
            TokenKind::AndAnd => Token::AndAnd,
            TokenKind::OrOr => Token::OrOr,
            TokenKind::Arrow => Token::Arrow,
            TokenKind::Assign => Token::Assign,
            TokenKind::Lt => Token::Lt,
            TokenKind::Gt => Token::Gt,
            TokenKind::Plus => Token::Plus,
            TokenKind::Minus => Token::Minus,
            TokenKind::Star => Token::Star,
            TokenKind::Slash => Token::Slash,
            TokenKind::Percent => Token::Percent,
            TokenKind::Bang => Token::Bang,
        })
    }
}

fn unescape_string(quoted: &str) -> Result<String, LexerError> {
    let inner = quoted
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| LexerError::InvalidString {
            line: 0,
            column: 0,
            message: "missing quotes".into(),
        })?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => {
                    return Err(LexerError::InvalidString {
                        line: 0,
                        column: 0,
                        message: "trailing escape".into(),
                    })
                }
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}
