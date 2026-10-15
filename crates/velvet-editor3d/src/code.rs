//! Syntax colours for the script editor: a small hand-written scanner per language (Rust, Velvet
//! Script, JSON, Markdown guiones, TOML, WGSL, JS), enough to read code at a glance.

use std::path::Path;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Vel,
    Json,
    Markdown,
    Toml,
    Wgsl,
    Js,
    Plain,
}

impl Lang {
    pub fn of(path: &Path) -> Lang {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let ext = name.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => Lang::Rust,
            "vel" | "vs3" | "vcss" => Lang::Vel,
            "json" => Lang::Json,
            "md" => Lang::Markdown,
            "toml" => Lang::Toml,
            "wgsl" => Lang::Wgsl,
            "js" | "mjs" | "ts" => Lang::Js,
            _ => Lang::Plain,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "Rust",
            Lang::Vel => "Velvet Script",
            Lang::Json => "JSON",
            Lang::Markdown => "Markdown / guion",
            Lang::Toml => "TOML",
            Lang::Wgsl => "WGSL",
            Lang::Js => "JavaScript",
            Lang::Plain => "Texto",
        }
    }
    /// Whether the editor should open files with this name as text.
    pub fn is_text(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let ext = name.rsplit('.').next().unwrap_or("");
        matches!(
            ext,
            "rs" | "vel"
                | "vs3"
                | "vcss"
                | "json"
                | "md"
                | "toml"
                | "wgsl"
                | "js"
                | "mjs"
                | "ts"
                | "txt"
                | "ron"
                | "yaml"
                | "yml"
                | "html"
                | "css"
                | "py"
                | "ps1"
                | "sh"
                | "cfg"
                | "ini"
                | "csv"
                | "glsl"
                | "frag"
                | "vert"
        )
    }
}

const C_TEXT: Color32 = Color32::from_rgb(0xd7, 0xda, 0xe2);
const C_KEYWORD: Color32 = Color32::from_rgb(0xff, 0x70, 0x8a);
const C_CONTROL: Color32 = Color32::from_rgb(0xff, 0x8c, 0xcc);
const C_STRING: Color32 = Color32::from_rgb(0xff, 0xe0, 0x8a);
const C_NUMBER: Color32 = Color32::from_rgb(0xa1, 0xff, 0xe0);
const C_COMMENT: Color32 = Color32::from_rgb(0x6c, 0x74, 0x84);
const C_TYPE: Color32 = Color32::from_rgb(0x8e, 0xf0, 0xd0);
const C_FUNC: Color32 = Color32::from_rgb(0x66, 0xb8, 0xff);
const C_KEY: Color32 = Color32::from_rgb(0x9a, 0xc4, 0xff);
const C_HEADING: Color32 = Color32::from_rgb(0xb8, 0x8c, 0xff);
const C_DIRECTIVE: Color32 = Color32::from_rgb(0xff, 0xb3, 0x4a);
const C_QUOTE: Color32 = Color32::from_rgb(0x9b, 0xa3, 0xb3);
const C_SYMBOL: Color32 = Color32::from_rgb(0xab, 0xc9, 0xff);

fn keywords(lang: Lang) -> (&'static [&'static str], &'static [&'static str]) {
    match lang {
        Lang::Rust => (
            &[
                "fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "use", "mod",
                "crate", "self", "Self", "super", "const", "static", "type", "where", "as", "dyn",
                "ref", "move", "unsafe", "extern", "true", "false", "async", "await",
            ],
            &[
                "if", "else", "match", "for", "while", "loop", "return", "break", "continue", "in",
            ],
        ),
        Lang::Vel => (
            &[
                "fn", "function", "let", "var", "const", "struct", "enum", "import", "export",
                "from", "scene", "screen", "true", "false", "null", "self", "map", "list", "any",
                "int", "float", "string", "bool",
            ],
            &[
                "if", "else", "match", "for", "while", "loop", "return", "break", "continue", "in",
            ],
        ),
        Lang::Wgsl => (
            &[
                "fn",
                "let",
                "var",
                "const",
                "struct",
                "uniform",
                "storage",
                "read",
                "write",
                "vec2",
                "vec3",
                "vec4",
                "f32",
                "u32",
                "i32",
                "mat4x4",
                "mat3x3",
                "texture_2d",
                "sampler",
                "true",
                "false",
                "override",
            ],
            &[
                "if", "else", "for", "while", "loop", "return", "break", "continue", "discard",
                "switch", "case",
            ],
        ),
        Lang::Js => (
            &[
                "function",
                "const",
                "let",
                "var",
                "class",
                "import",
                "export",
                "from",
                "new",
                "this",
                "true",
                "false",
                "null",
                "undefined",
                "async",
                "await",
                "of",
                "typeof",
            ],
            &[
                "if", "else", "switch", "case", "for", "while", "do", "return", "break",
                "continue", "in", "try", "catch", "finally", "throw",
            ],
        ),
        Lang::Json | Lang::Toml => (&["true", "false", "null"], &[]),
        Lang::Markdown | Lang::Plain => (&[], &[]),
    }
}

fn push(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
}

/// The coloured layout of `text` in `lang`.
pub fn highlight(text: &str, lang: Lang, font: &FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    match lang {
        Lang::Markdown => markdown(&mut job, text, font),
        Lang::Plain => push(&mut job, text, font, C_TEXT),
        _ => code(&mut job, text, lang, font),
    }
    job
}

fn markdown(job: &mut LayoutJob, text: &str, font: &FontId) {
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        let color = if t.starts_with('#') {
            Some(C_HEADING)
        } else if t.starts_with('>') {
            Some(C_QUOTE)
        } else if t.starts_with('@') {
            Some(C_DIRECTIVE)
        } else if t.starts_with("---") || t.starts_with("<!--") {
            Some(C_COMMENT)
        } else {
            None
        };
        if let Some(c) = color {
            push(job, line, font, c);
            continue;
        }
        // Inline: **bold** names, `code`, {attributes}.
        let mut rest = line;
        while !rest.is_empty() {
            if let Some(i) = rest.find(['*', '`', '{']) {
                push(job, &rest[..i], font, C_TEXT);
                let tail = &rest[i..];
                let (open, close, color) = if tail.starts_with("**") {
                    ("**", "**", C_KEY)
                } else if tail.starts_with('`') {
                    ("`", "`", C_STRING)
                } else if tail.starts_with('{') {
                    ("{", "}", C_DIRECTIVE)
                } else {
                    ("*", "*", C_QUOTE)
                };
                if let Some(end) = tail[open.len()..].find(close) {
                    let stop = open.len() + end + close.len();
                    push(job, &tail[..stop], font, color);
                    rest = &tail[stop..];
                } else {
                    push(job, tail, font, C_TEXT);
                    rest = "";
                }
            } else {
                push(job, rest, font, C_TEXT);
                rest = "";
            }
        }
    }
}

fn code(job: &mut LayoutJob, text: &str, lang: Lang, font: &FontId) {
    let (kw, control) = keywords(lang);
    let line_comment: &[&str] = match lang {
        Lang::Toml => &["#"],
        Lang::Json => &[],
        _ => &["//"],
    };
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;
    let flush =
        |job: &mut LayoutJob, from: usize, to: usize| push(job, &text[from..to], font, C_TEXT);
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Comments.
        if line_comment.iter().any(|m| text[i..].starts_with(m)) {
            flush(job, plain_start, i);
            let end = text[i..].find('\n').map_or(text.len(), |e| i + e);
            push(job, &text[i..end], font, C_COMMENT);
            i = end;
            plain_start = i;
            continue;
        }
        if lang != Lang::Toml && lang != Lang::Json && text[i..].starts_with("/*") {
            flush(job, plain_start, i);
            let end = text[i + 2..]
                .find("*/")
                .map_or(text.len(), |e| i + 2 + e + 2);
            push(job, &text[i..end], font, C_COMMENT);
            i = end;
            plain_start = i;
            continue;
        }
        // Strings.
        if c == '"'
            || (c == '\'' && matches!(lang, Lang::Js | Lang::Vel))
            || (c == '`' && lang == Lang::Js)
        {
            flush(job, plain_start, i);
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] as char != c {
                if bytes[j] == b'\\' {
                    j += 1;
                }
                if bytes[j.min(bytes.len() - 1)] == b'\n' && c != '`' {
                    break;
                }
                j += 1;
            }
            let end = (j + 1).min(bytes.len());
            let s = &text[i..end];
            // A JSON key (a string followed by a colon) reads differently from a value.
            let after = text[end..].trim_start_matches([' ', '\t']);
            let color = if lang == Lang::Json && after.starts_with(':') {
                C_KEY
            } else {
                C_STRING
            };
            push(job, s, font, color);
            i = end;
            plain_start = i;
            continue;
        }
        // Numbers.
        if c.is_ascii_digit()
            && (i == 0 || !(bytes[i - 1] as char).is_ascii_alphanumeric() && bytes[i - 1] != b'_')
        {
            flush(job, plain_start, i);
            let mut j = i;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric()
                    || bytes[j] == b'.'
                    || bytes[j] == b'_')
            {
                j += 1;
            }
            push(job, &text[i..j], font, C_NUMBER);
            i = j;
            plain_start = i;
            continue;
        }
        // Words.
        if c.is_ascii_alphabetic() || c == '_' || c == '@' || c == '#' && lang == Lang::Rust {
            let mut j = i + 1;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            let word = &text[i..j];
            let next = text[j..].trim_start_matches(' ');
            let color = if c == '@' || c == '#' {
                Some(C_DIRECTIVE)
            } else if control.contains(&word) {
                Some(C_CONTROL)
            } else if kw.contains(&word) {
                Some(C_KEYWORD)
            } else if lang == Lang::Toml && next.starts_with('=') {
                Some(C_KEY)
            } else if next.starts_with('(') || next.starts_with("!(") {
                Some(C_FUNC)
            } else if word.chars().next().is_some_and(|f| f.is_ascii_uppercase()) {
                Some(C_TYPE)
            } else {
                None
            };
            if let Some(color) = color {
                flush(job, plain_start, i);
                push(job, word, font, color);
                plain_start = j;
            }
            i = j;
            continue;
        }
        if lang == Lang::Toml && c == '[' && (i == 0 || bytes[i - 1] == b'\n') {
            flush(job, plain_start, i);
            let end = text[i..].find('\n').map_or(text.len(), |e| i + e);
            push(job, &text[i..end], font, C_HEADING);
            i = end;
            plain_start = i;
            continue;
        }
        if "{}[]()<>=+-*/%!&|^:;,.".contains(c) {
            flush(job, plain_start, i);
            push(job, &text[i..i + 1], font, C_SYMBOL);
            i += 1;
            plain_start = i;
            continue;
        }
        // Advance by one character (UTF-8 safe).
        i += text[i..].chars().next().map_or(1, char::len_utf8);
    }
    flush(job, plain_start, text.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighting_keeps_every_character() {
        let font = FontId::monospace(13.0);
        for (src, lang) in [
            (
                "fn main() { let x = 42; // hi\n println!(\"ñ{x}\"); }",
                Lang::Rust,
            ),
            ("{\"name\": \"plaza\", \"n\": [1, 2.5, true]}", Lang::Json),
            (
                "## saludo {quien=lira}\n**Lira:** —¿Qué `x`?\n@sala trono\n",
                Lang::Markdown,
            ),
            ("[workspace]\nname = \"x\" # c\n", Lang::Toml),
            ("const a = 'b'; // ü\n", Lang::Js),
        ] {
            let job = highlight(src, lang, &font);
            assert_eq!(job.text, src, "{lang:?}");
        }
    }
}
