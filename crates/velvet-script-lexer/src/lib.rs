//! # velvet-script-lexer
//!
//! Tokenizes Velvet Script source into a stream of [`Token`] values with spans.

#![deny(missing_docs)]

mod token;

pub use token::{LexerError, Span, Token, TokenKind};

use logos::Logos;

/// A token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct LexedToken {
    /// Kind / payload.
    pub token: Token,
    /// Byte span in the source.
    pub span: Span,
    /// 1-based line.
    pub line: u32,
    /// 1-based column (char offset on line, approximate UTF-8).
    pub column: u32,
}

/// Lex full source into tokens (skips whitespace/comments).
pub fn lex(source: &str) -> Result<Vec<LexedToken>, LexerError> {
    let mut out = Vec::new();
    let mut line_starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }

    let mut lexer = TokenKind::lexer(source);
    while let Some(result) = lexer.next() {
        let span = Span {
            start: lexer.span().start,
            end: lexer.span().end,
        };
        let (line, column) = line_col(&line_starts, span.start);
        match result {
            Ok(TokenKind::Comment) => continue,
            Ok(kind) => {
                let token = kind.into_token(lexer.slice())?;
                out.push(LexedToken {
                    token,
                    span,
                    line,
                    column,
                });
            }
            Err(()) => {
                return Err(LexerError::Unexpected {
                    line,
                    column,
                    snippet: lexer.slice().chars().take(16).collect(),
                });
            }
        }
    }
    Ok(out)
}

fn line_col(line_starts: &[usize], offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut start = 0usize;
    for (i, &s) in line_starts.iter().enumerate() {
        if s <= offset {
            line = (i as u32) + 1;
            start = s;
        } else {
            break;
        }
    }
    let column = (offset - start) as u32 + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_character_and_dialogue() {
        let src = r#"
character aria {
    name: "Aria"
}

aria "Hello"
"#;
        let tokens = lex(src).unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.token, Token::Ident(ref s) if s == "character")));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.token, Token::Ident(ref s) if s == "aria")));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.token, Token::String(ref s) if s == "Aria")));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.token, Token::String(ref s) if s == "Hello")));
    }

    #[test]
    fn lex_numbers_and_ops() {
        let tokens = lex("aria_trust += 1").unwrap();
        assert!(matches!(tokens[0].token, Token::Ident(_)));
        assert_eq!(tokens[1].token, Token::PlusAssign);
        assert_eq!(tokens[2].token, Token::Int(1));
    }

    #[test]
    fn error_reports_line_column() {
        let err = lex("let x = @").unwrap_err();
        match err {
            LexerError::Unexpected { line, column, .. } => {
                assert_eq!(line, 1);
                assert!(column >= 9);
            }
            _ => panic!("expected unexpected"),
        }
    }

    #[test]
    fn skips_comments() {
        let tokens = lex("// hi\n1 + 2").unwrap();
        assert_eq!(tokens[0].token, Token::Int(1));
        assert_eq!(tokens[1].token, Token::Plus);
        assert_eq!(tokens[2].token, Token::Int(2));
    }

    #[test]
    fn keywords_as_idents_when_needed() {
        // Velvet uses contextual keywords; lexer emits Ident for words.
        let tokens = lex("scene choice jump").unwrap();
        assert_eq!(tokens.len(), 3);
    }
}
