//! VS2 formatter rules — rust-like braces, no Python significant whitespace.

#![allow(missing_docs)]
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct Vs2FormatOptions {
    pub indent_width: usize,
    pub use_tabs: bool,
    pub max_width: usize,
    pub trailing_comma: bool,
    pub space_before_brace: bool,
    pub newline_eof: bool,
}

impl Default for Vs2FormatOptions {
    fn default() -> Self {
        Self { indent_width: 4, use_tabs: false, max_width: 100,
               trailing_comma: true, space_before_brace: true, newline_eof: true }
    }
}

impl Vs2FormatOptions {
    pub fn indent_str(&self, level: usize) -> String {
        if self.use_tabs { "\t".repeat(level) } else { " ".repeat(self.indent_width * level) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vs2TokKind {
    Ident, Number, String, LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Comma, Semi, Colon, PathSep, Op, Comment, Newline, Other,
}

#[derive(Debug, Clone)]
pub struct Vs2Tok { pub kind: Vs2TokKind, pub text: String }

pub fn lex_format(src: &str) -> Vec<Vs2Tok> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c == '\n' {
            out.push(Vs2Tok { kind: Vs2TokKind::Newline, text: "\n".into() });
            i += 1; continue;
        }
        if c.is_whitespace() { i += 1; continue; }
        if c == '/' && i + 1 < b.len() && b[i + 1] as char == '/' {
            let start = i; i += 2;
            while i < b.len() && b[i] as char != '\n' { i += 1; }
            out.push(Vs2Tok { kind: Vs2TokKind::Comment, text: src[start..i].to_string() });
            continue;
        }
        if c == '"' {
            let start = i; i += 1;
            while i < b.len() {
                if b[i] as char == '\\' && i + 1 < b.len() { i += 2; continue; }
                if b[i] as char == '"' { i += 1; break; }
                i += 1;
            }
            out.push(Vs2Tok { kind: Vs2TokKind::String, text: src[start..i].to_string() });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i; i += 1;
            while i < b.len() {
                let ch = b[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' { i += 1; } else { break; }
            }
            out.push(Vs2Tok { kind: Vs2TokKind::Ident, text: src[start..i].to_string() });
            continue;
        }
        if c.is_ascii_digit() {
            let start = i; i += 1;
            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] as char == '.') { i += 1; }
            out.push(Vs2Tok { kind: Vs2TokKind::Number, text: src[start..i].to_string() });
            continue;
        }
        if c == ':' && i + 1 < b.len() && b[i + 1] as char == ':' {
            out.push(Vs2Tok { kind: Vs2TokKind::PathSep, text: "::".into() });
            i += 2; continue;
        }
        let (kind, text) = match c {
            '{' => (Vs2TokKind::LBrace, "{"), '}' => (Vs2TokKind::RBrace, "}"),
            '(' => (Vs2TokKind::LParen, "("), ')' => (Vs2TokKind::RParen, ")"),
            '[' => (Vs2TokKind::LBracket, "["), ']' => (Vs2TokKind::RBracket, "]"),
            ',' => (Vs2TokKind::Comma, ","), ';' => (Vs2TokKind::Semi, ";"),
            ':' => (Vs2TokKind::Colon, ":"), _ => (Vs2TokKind::Op, ""),
        };
        if kind == Vs2TokKind::Op {
            out.push(Vs2Tok { kind, text: c.to_string() });
        } else {
            out.push(Vs2Tok { kind, text: text.into() });
        }
        i += 1;
    }
    out
}

pub fn format_vs2(src: &str, opt: &Vs2FormatOptions) -> String {
    let toks = lex_format(src);
    let mut out = String::new();
    let mut level: i32 = 0;
    let mut at_line_start = true;
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        match t.kind {
            Vs2TokKind::Newline => { out.push('\n'); at_line_start = true; }
            Vs2TokKind::RBrace => {
                level = (level - 1).max(0);
                if at_line_start { out.push_str(&opt.indent_str(level as usize)); }
                out.push('}');
                at_line_start = false;
            }
            Vs2TokKind::LBrace => {
                if opt.space_before_brace && !at_line_start && !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
                out.push('{');
                level += 1;
                at_line_start = false;
            }
            Vs2TokKind::Comment => {
                if at_line_start { out.push_str(&opt.indent_str(level as usize)); }
                out.push_str(&t.text);
                at_line_start = false;
            }
            _ => {
                if at_line_start {
                    out.push_str(&opt.indent_str(level as usize));
                    at_line_start = false;
                } else if needs_space_before(&toks, i) {
                    out.push(' ');
                }
                out.push_str(&t.text);
            }
        }
        i += 1;
    }
    if opt.newline_eof && !out.ends_with('\n') { out.push('\n'); }
    out
}

fn needs_space_before(toks: &[Vs2Tok], i: usize) -> bool {
    if i == 0 { return false; }
    let prev = &toks[i - 1];
    let cur = &toks[i];
    if matches!(prev.kind, Vs2TokKind::LParen | Vs2TokKind::LBracket | Vs2TokKind::PathSep) { return false; }
    if matches!(cur.kind, Vs2TokKind::RParen | Vs2TokKind::RBracket | Vs2TokKind::Comma | Vs2TokKind::Semi | Vs2TokKind::Colon) { return false; }
    if prev.kind == Vs2TokKind::Ident && cur.kind == Vs2TokKind::LParen { return false; }
    if prev.kind == Vs2TokKind::Ident && cur.kind == Vs2TokKind::PathSep { return false; }
    if prev.kind == Vs2TokKind::PathSep { return false; }
    true
}

pub fn looks_like_python(src: &str) -> bool {
    let has_brace = src.contains('{');
    let indent_lines = src.lines().filter(|l| l.starts_with("    ") || l.starts_with('\t')).count();
    !has_brace && indent_lines > 3 && (src.contains("def ") || src.contains("elif "))
}

/// Sample used by tests (not N clones).
pub fn format_sample_source() -> &'static str {
    "// @edition 2\nfn main() {\n    let x = 1;\n    return x;\n}\n"
}

pub fn reject_python_style(src: &str) -> Result<(), String> {
    if looks_like_python(src) {
        Err("Velvet Script 2 is not Python: use braces `{}`, typed fn/struct, not def/elif indent".into())
    } else { Ok(()) }
}
























































#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn braces_indent() {
        let src = "fn main(){\nlet x=1;\n}";
        let out = format_vs2(src, &Vs2FormatOptions::default());
        assert!(out.contains("fn main()"));
        assert!(out.contains('{'));
    }
    #[test]
    fn rejects_python() {
        let py = "def foo():\n    x = 1\n    if x:\n        y = 2\n    elif x:\n        y = 3\n";
        assert!(looks_like_python(py));
        assert!(reject_python_style(py).is_err());
    }
    #[test]
    fn format_sample_fn() {
        let src = "fn main(){
let x=1;
}
";
        let out = format_vs2(src, &Vs2FormatOptions::default());
        assert!(out.contains("fn main()"));
        assert!(out.contains('{'));
        let twice = format_vs2(&out, &Vs2FormatOptions::default());
        assert_eq!(out, twice);
    }
}

