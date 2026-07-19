//! Indent-sensitive lexer for Velvet Story (UTF-8 safe).

use crate::diag::StoryDiag;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Lex result.
#[derive(Debug)]
pub struct LexResult {
    /// Tokens (with Indent/Dedent/Newline).
    pub tokens: Vec<Token>,
    /// Diagnostics.
    pub diags: Vec<StoryDiag>,
}

/// Lex a Velvet Story source file.
pub fn lex(source: &str, file: &str) -> LexResult {
    let mut tokens = Vec::new();
    let mut diags = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut tab_seen = false;
    let mut space_seen = false;

    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut ci = 0usize; // index into chars
    let mut line: u32 = 1;
    let mut col: u32 = 1;
    let mut at_line_start = true;

    let byte_at = |ci: usize| -> usize {
        if ci >= chars.len() {
            source.len()
        } else {
            chars[ci].0
        }
    };

    while ci < chars.len() {
        if at_line_start {
            let start_b = byte_at(ci);
            let start_col = col;
            let mut indent = 0usize;
            while ci < chars.len() {
                match chars[ci].1 {
                    ' ' => {
                        space_seen = true;
                        indent += 1;
                        ci += 1;
                        col += 1;
                    }
                    '\t' => {
                        tab_seen = true;
                        indent += 4;
                        ci += 1;
                        col += 1;
                    }
                    _ => break,
                }
            }
            if tab_seen && space_seen {
                diags.push(StoryDiag::error_key(
                    "VST001",
                    &[],
                    file,
                    Span::at(line, start_col, start_b, byte_at(ci)),
                ));
            }
            if ci < chars.len()
                && (chars[ci].1 == '\n' || chars[ci].1 == '\r' || chars[ci].1 == '#')
            {
                at_line_start = false;
            } else {
                let cur = *indent_stack.last().unwrap_or(&0);
                if indent > cur {
                    indent_stack.push(indent);
                    tokens.push(Token::new(
                        TokenKind::Indent(indent),
                        Span::at(line, start_col, start_b, byte_at(ci)),
                    ));
                } else {
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > indent {
                        indent_stack.pop();
                        tokens.push(Token::new(
                            TokenKind::Dedent,
                            Span::at(line, start_col, start_b, byte_at(ci)),
                        ));
                    }
                    if *indent_stack.last().unwrap_or(&0) != indent {
                        let expected = indent_stack.last().unwrap_or(&0).to_string();
                        let found = indent.to_string();
                        diags.push(StoryDiag::error_key(
                            "VST002",
                            &[("expected", expected.as_str()), ("indent", found.as_str())],
                            file,
                            Span::at(line, start_col, start_b, byte_at(ci)),
                        ));
                        indent_stack.push(indent);
                    }
                }
                at_line_start = false;
            }
            continue;
        }

        let (byte_start, c) = chars[ci];
        let start_col = col;

        if c == '\r' {
            ci += 1;
            continue;
        }
        if c == '\n' {
            tokens.push(Token::new(
                TokenKind::Newline,
                Span::at(line, col, byte_start, byte_start + 1),
            ));
            ci += 1;
            line += 1;
            col = 1;
            at_line_start = true;
            continue;
        }

        if c == '#' {
            let s = byte_start;
            while ci < chars.len() && chars[ci].1 != '\n' {
                ci += 1;
                col += 1;
            }
            let e = byte_at(ci);
            tokens.push(Token::new(
                TokenKind::Comment(source[s..e].to_string()),
                Span::at(line, start_col, s, e),
            ));
            continue;
        }

        if c.is_whitespace() {
            ci += 1;
            col += 1;
            continue;
        }

        if c == '"' {
            ci += 1;
            col += 1;
            let mut out = String::new();
            while ci < chars.len() {
                let ch = chars[ci].1;
                if ch == '\\' && ci + 1 < chars.len() {
                    let n = chars[ci + 1].1;
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        o => o,
                    });
                    ci += 2;
                    col += 2;
                    continue;
                }
                if ch == '"' {
                    ci += 1;
                    col += 1;
                    break;
                }
                out.push(ch);
                ci += 1;
                col += 1;
            }
            tokens.push(Token::new(
                TokenKind::String(out),
                Span::at(line, start_col, byte_start, byte_at(ci)),
            ));
            continue;
        }

        if c.is_ascii_digit() {
            let s = byte_start;
            ci += 1;
            col += 1;
            let mut is_float = false;
            while ci < chars.len() {
                let ch = chars[ci].1;
                if ch.is_ascii_digit() {
                    ci += 1;
                    col += 1;
                } else if ch == '.' && !is_float {
                    is_float = true;
                    ci += 1;
                    col += 1;
                } else {
                    break;
                }
            }
            let e = byte_at(ci);
            let text = &source[s..e];
            if is_float {
                tokens.push(Token::new(
                    TokenKind::Float(text.to_string()),
                    Span::at(line, start_col, s, e),
                ));
            } else {
                let n: i64 = text.parse().unwrap_or(0);
                tokens.push(Token::new(
                    TokenKind::Int(n),
                    Span::at(line, start_col, s, e),
                ));
            }
            continue;
        }

        // two-char ops
        if ci + 1 < chars.len() {
            let two: String = chars[ci..ci + 2].iter().map(|(_, ch)| *ch).collect();
            let kind = match two.as_str() {
                "==" => Some(TokenKind::EqEq),
                "!=" => Some(TokenKind::Ne),
                "<=" => Some(TokenKind::Le),
                ">=" => Some(TokenKind::Ge),
                _ => None,
            };
            if let Some(k) = kind {
                tokens.push(Token::new(
                    k,
                    Span::at(line, start_col, byte_start, byte_at(ci + 2)),
                ));
                ci += 2;
                col += 2;
                continue;
            }
        }

        let single = match c {
            ':' => Some(TokenKind::Colon),
            '@' => Some(TokenKind::At),
            '=' => Some(TokenKind::Eq),
            '<' => Some(TokenKind::Lt),
            '>' => Some(TokenKind::Gt),
            '+' => Some(TokenKind::Plus),
            '-' => Some(TokenKind::Minus),
            '*' => Some(TokenKind::Star),
            '/' => Some(TokenKind::Slash),
            _ => None,
        };
        if let Some(k) = single {
            tokens.push(Token::new(
                k,
                Span::at(line, start_col, byte_start, byte_at(ci + 1)),
            ));
            ci += 1;
            col += 1;
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let s = byte_start;
            ci += 1;
            col += 1;
            while ci < chars.len() {
                let ch = chars[ci].1;
                if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                    ci += 1;
                    col += 1;
                } else {
                    break;
                }
            }
            let e = byte_at(ci);
            let text = &source[s..e];
            let kind = match text {
                "and" => TokenKind::And,
                "or" => TokenKind::Or,
                "not" => TokenKind::Not,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Ident(text.to_string()),
            };
            tokens.push(Token::new(kind, Span::at(line, start_col, s, e)));
            continue;
        }

        // Dialogue-friendly: absorb punctuation / unicode into a bare word
        // so writers can type “¿Hola?” without escaping.
        let s = byte_start;
        while ci < chars.len() {
            let ch = chars[ci].1;
            if ch.is_whitespace()
                || matches!(
                    ch,
                    '\n' | '\r' | '#' | '"' | ':' | '@' | '=' | '<' | '>' | '+' | '-' | '*' | '/'
                )
            {
                break;
            }
            ci += 1;
            col += 1;
        }
        let e = byte_at(ci);
        if e > s {
            tokens.push(Token::new(
                TokenKind::Ident(source[s..e].to_string()),
                Span::at(line, start_col, s, e),
            ));
        } else {
            let ch = format!("{c:?}");
            diags.push(StoryDiag::error_key(
                "VST003",
                &[("ch", ch.as_str())],
                file,
                Span::at(line, start_col, byte_start, byte_at(ci + 1)),
            ));
            ci += 1;
            col += 1;
        }
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token::new(
            TokenKind::Dedent,
            Span::at(line, col, source.len(), source.len()),
        ));
    }
    tokens.push(Token::new(
        TokenKind::Eof,
        Span::at(line, col, source.len(), source.len()),
    ));

    LexResult { tokens, diags }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple_scene() {
        let src = "scene start\n\nluna:\n    Hola\n";
        let r = lex(src, "t.vstory");
        assert!(r
            .tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "scene")));
    }

    #[test]
    fn lex_utf8_dialogue() {
        let src = "scene a\nluna:\n    ¿Dormiste bien?\n";
        let r = lex(src, "u.vstory");
        assert!(!r
            .diags
            .iter()
            .any(|d| d.code == "VST003" && d.message.contains("boundary")));
    }
}
