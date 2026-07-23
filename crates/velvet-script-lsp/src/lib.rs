//! # velvet-script-lsp
//!
//! Language intelligence for Velvet Script: analysis APIs plus **LSP stdio**
//! (`stdio` module) for VS Code and Velvet Studio.

#![deny(missing_docs)]

pub mod stdio;

use serde::{Deserialize, Serialize};
use velvet_script_ast::Item;
use velvet_script_bytecode::{lookup_math_constant, lookup_native, NativeId, NativePurity};
use velvet_script_compiler::compile_source;
use velvet_script_parser::parse_file;

/// Diagnostic severity for editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Error.
    Error,
    /// Warning.
    Warning,
    /// Info.
    Information,
    /// Hint.
    Hint,
}

/// LSP-like diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// 0-based line.
    pub line: u32,
    /// 0-based character.
    pub character: u32,
    /// End line.
    pub end_line: u32,
    /// End character.
    pub end_character: u32,
    /// Message.
    pub message: String,
    /// Severity.
    pub severity: Severity,
    /// Source tool.
    pub source: String,
}

/// Document symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSymbol {
    /// Name.
    pub name: String,
    /// Kind: function, scene, screen, character, variable.
    pub kind: String,
    /// Line 0-based.
    pub line: u32,
    /// Character.
    pub character: u32,
}

/// Analysis result for a document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Analysis {
    /// Diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Symbols.
    pub symbols: Vec<DocumentSymbol>,
}

/// A range in the document (0-based line/character, end exclusive-ish).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    /// Start line.
    pub line: u32,
    /// Start character.
    pub character: u32,
    /// End line.
    pub end_line: u32,
    /// End character.
    pub end_character: u32,
}

/// A text edit for rename / code actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace.
    pub range: TextRange,
    /// Replacement text.
    pub new_text: String,
}

/// Hover information for a symbol or keyword.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoverInfo {
    /// Display range.
    pub range: TextRange,
    /// Markdown-ish contents.
    pub contents: String,
    /// Symbol kind when known.
    pub kind: Option<String>,
}

/// Semantic token kinds (simplified).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticTokenKind {
    /// Language keyword.
    Keyword,
    /// Identifier / symbol.
    Identifier,
    /// Function name at definition or call.
    Function,
    /// String literal.
    String,
    /// Number literal.
    Number,
    /// Comment (reserved; lexer may not surface).
    Comment,
}

/// A simple semantic token span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticToken {
    /// Range.
    pub range: TextRange,
    /// Kind.
    pub kind: SemanticTokenKind,
    /// Token text.
    pub text: String,
}

/// Analyze Velvet Script source for diagnostics and outline symbols.
pub fn analyze(source: &str, file: Option<&str>) -> Analysis {
    let mut analysis = Analysis::default();
    let parsed = match parse_file(source, file) {
        Ok(p) => p,
        Err(e) => {
            analysis.diagnostics.push(Diagnostic {
                line: 0,
                character: 0,
                end_line: 0,
                end_character: 1,
                message: e.to_string(),
                severity: Severity::Error,
                source: "velvet-parser".into(),
            });
            return analysis;
        }
    };

    for d in &parsed.module.diagnostics {
        let line = d.loc.line.saturating_sub(1);
        let character = d.loc.column.saturating_sub(1);
        analysis.diagnostics.push(Diagnostic {
            line,
            character,
            end_line: line,
            end_character: character + 1,
            message: d.message.clone(),
            severity: Severity::Error,
            source: "velvet-parser".into(),
        });
    }

    for item in &parsed.module.items {
        match item {
            Item::Import { path, alias, loc } => analysis.symbols.push(DocumentSymbol {
                name: alias.clone().unwrap_or_else(|| path.clone()),
                kind: "module".into(),
                line: loc.line.saturating_sub(1),
                character: loc.column.saturating_sub(1),
            }),
            Item::Function { name, loc, .. } => analysis.symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: "function".into(),
                line: loc.line.saturating_sub(1),
                character: loc.column.saturating_sub(1),
            }),
            Item::Scene { name, loc, .. } => analysis.symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: "scene".into(),
                line: loc.line.saturating_sub(1),
                character: loc.column.saturating_sub(1),
            }),
            Item::Screen { name, loc, .. } => analysis.symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: "screen".into(),
                line: loc.line.saturating_sub(1),
                character: loc.column.saturating_sub(1),
            }),
            Item::Character { name, loc, .. } => analysis.symbols.push(DocumentSymbol {
                name: name.clone(),
                kind: "character".into(),
                line: loc.line.saturating_sub(1),
                character: loc.column.saturating_sub(1),
            }),
            Item::State { bindings, .. } => {
                for b in bindings {
                    analysis.symbols.push(DocumentSymbol {
                        name: b.name.clone(),
                        kind: "variable".into(),
                        line: b.loc.line.saturating_sub(1),
                        character: b.loc.column.saturating_sub(1),
                    });
                }
            }
            Item::Stmt(_) => {}
        }
    }

    if velvet_script_vs3::is_vs3_source(source) {
        if let Err(error) = velvet_script_vs3::compile(source, file) {
            if error.diagnostics().is_empty() {
                analysis.diagnostics.push(Diagnostic {
                    line: 0,
                    character: 0,
                    end_line: 0,
                    end_character: 1,
                    message: error.to_string(),
                    severity: Severity::Error,
                    source: "velvet-vs3".into(),
                });
            } else {
                for diagnostic in error.diagnostics() {
                    let line = diagnostic.loc.line.saturating_sub(1);
                    let character = diagnostic.loc.column.saturating_sub(1);
                    analysis.diagnostics.push(Diagnostic {
                        line,
                        character,
                        end_line: line,
                        end_character: character.saturating_add(1),
                        message: diagnostic.message.clone(),
                        severity: Severity::Error,
                        source: "velvet-vs3".into(),
                    });
                }
            }
        }
    } else if let Err(e) = compile_source(source, file) {
        // Prefer structured multi-error when available.
        let diags = e.diagnostics();
        if !diags.is_empty() {
            for d in diags {
                let line = d.loc.line.saturating_sub(1);
                let character = d.loc.column.saturating_sub(1);
                analysis.diagnostics.push(Diagnostic {
                    line,
                    character,
                    end_line: line,
                    end_character: character.saturating_add(1),
                    message: d.message.clone(),
                    severity: Severity::Error,
                    source: "velvet-compiler".into(),
                });
            }
        } else {
            analysis.diagnostics.push(Diagnostic {
                line: 0,
                character: 0,
                end_line: 0,
                end_character: 1,
                message: e.to_string(),
                severity: Severity::Error,
                source: "velvet-compiler".into(),
            });
        }
    }

    analysis
}

/// Completion items at a cursor (keyword / symbol heuristic).
pub fn completions(source: &str, _line: u32, _character: u32) -> Vec<String> {
    let mut items: Vec<String> = if velvet_script_vs3::is_vs3_source(source) {
        let mut items = [
            "import", "export", "function", "state", "return", "let", "const", "if", "else",
            "while", "for", "in", "break", "continue", "yield", "null", "true", "false", "int",
            "float", "bool", "string", "list", "map", "vec2", "vec3", "vec4", "mat3", "mat4",
            "quat", "rng", "PI", "TAU", "E", "EPSILON", "INFINITY", "NAN",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        items.extend(
            NativeId::all()
                .iter()
                .map(|native| native.name().to_string()),
        );
        items
    } else {
        [
            "function",
            "scene",
            "screen",
            "button",
            "character",
            "state",
            "choice",
            "jump",
            "label",
            "return",
            "let",
            "if",
            "while",
            "background",
            "music",
            "show",
            "abs",
            "min",
            "max",
            "floor",
            "ceil",
            "clamp",
            "len",
            "concat",
            "print",
            "str",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    };
    let analysis = analyze(source, None);
    for s in analysis.symbols {
        items.push(s.name);
    }
    items.sort();
    items.dedup();
    items
}

/// Go-to-definition: find first symbol matching word.
pub fn goto_definition(source: &str, word: &str) -> Option<DocumentSymbol> {
    analyze(source, None)
        .symbols
        .into_iter()
        .find(|s| s.name == word)
}

/// Find all textual references to `symbol` (definitions + identifier uses).
///
/// Uses word-boundary-aware scan of the source text so rename edits stay accurate
/// even when AST `SourceLoc` points at the start of a declaration keyword.
pub fn find_references(source: &str, symbol: &str) -> Vec<TextRange> {
    if symbol.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let sym: Vec<char> = symbol.chars().collect();
        if sym.is_empty() || chars.len() < sym.len() {
            continue;
        }
        let mut i = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        while i + sym.len() <= chars.len() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if chars[i] == '\\' {
                    escaped = true;
                } else if chars[i] == '"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }
            if chars[i] == '"' {
                in_string = true;
                i += 1;
                continue;
            }
            if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
                break;
            }
            if chars[i..i + sym.len()] == sym[..] {
                let before_ok =
                    i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
                let after_idx = i + sym.len();
                let after_ok = after_idx >= chars.len()
                    || !(chars[after_idx].is_ascii_alphanumeric() || chars[after_idx] == '_');
                if before_ok && after_ok {
                    ranges.push(TextRange {
                        line: line_idx as u32,
                        character: i as u32,
                        end_line: line_idx as u32,
                        end_character: after_idx as u32,
                    });
                }
                i = after_idx;
            } else {
                i += 1;
            }
        }
    }

    ranges.sort_by_key(|r| (r.line, r.character));
    ranges
}

/// Prepare rename: text edits replacing every reference of `symbol` with `new_name`.
pub fn rename_prepare(source: &str, symbol: &str, new_name: &str) -> Vec<TextEdit> {
    if symbol.is_empty() || new_name.is_empty() || symbol == new_name {
        return Vec::new();
    }
    find_references(source, symbol)
        .into_iter()
        .map(|range| TextEdit {
            range,
            new_text: new_name.to_string(),
        })
        .collect()
}

/// Apply rename edits to source (edits applied bottom-up so offsets stay valid).
pub fn apply_text_edits(source: &str, mut edits: Vec<TextEdit>) -> String {
    edits.sort_by(|a, b| {
        b.range
            .line
            .cmp(&a.range.line)
            .then(b.range.character.cmp(&a.range.character))
    });
    let mut lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();
    // Preserve trailing newline presence.
    let trailing_nl = source.ends_with('\n');
    for edit in edits {
        let line_idx = edit.range.line as usize;
        if line_idx >= lines.len() {
            continue;
        }
        let line = &lines[line_idx];
        let start = edit.range.character as usize;
        let end = edit.range.end_character as usize;
        if start > line.len() || end > line.len() || start > end {
            continue;
        }
        let mut new_line = String::new();
        new_line.push_str(&line[..start]);
        new_line.push_str(&edit.new_text);
        new_line.push_str(&line[end..]);
        lines[line_idx] = new_line;
    }
    let mut out = lines.join("\n");
    if trailing_nl {
        out.push('\n');
    }
    out
}

/// Hover info for the word at (line, character), 0-based.
pub fn hover(source: &str, line: u32, character: u32) -> Option<HoverInfo> {
    let word = word_at(source, line, character)?;
    let analysis = analyze(source, None);
    if let Some(sym) = analysis.symbols.iter().find(|s| s.name == word) {
        let range = TextRange {
            line: sym.line,
            character: sym.character,
            end_line: sym.line,
            end_character: sym.character + word.chars().count() as u32,
        };
        let contents = match sym.kind.as_str() {
            "function" => format!("```velvet\nfunction {word}(…)\n```\n\nFunction `{word}`"),
            "scene" => format!("```velvet\nscene {word}\n```\n\nScene `{word}`"),
            "character" => format!("```velvet\ncharacter {word}\n```\n\nCharacter `{word}`"),
            "variable" => format!("```velvet\n{word}\n```\n\nState variable `{word}`"),
            other => format!("`{word}` ({other})"),
        };
        return Some(HoverInfo {
            range,
            contents,
            kind: Some(sym.kind.clone()),
        });
    }
    // Keywords / stdlib.
    let kw = keyword_or_native_hover(&word)?;
    let range = TextRange {
        line,
        character: character.saturating_sub(
            word.chars()
                .count()
                .saturating_sub(1)
                .min(character as usize) as u32,
        ),
        end_line: line,
        end_character: character + 1,
    };
    // Better range: locate word on line.
    let range = word_range_on_line(source, line, &word).unwrap_or(range);
    Some(HoverInfo {
        range,
        contents: kw,
        kind: Some("keyword".into()),
    })
}

fn keyword_or_native_hover(word: &str) -> Option<String> {
    if let Some(native) = lookup_native(word) {
        let spec = native.spec();
        let arity = if spec.min_args == spec.max_args {
            spec.min_args.to_string()
        } else {
            format!("{}..={}", spec.min_args, spec.max_args)
        };
        let result = format!("{:?}", spec.result).to_ascii_lowercase();
        let purity = match spec.purity {
            NativePurity::Pure => "pure",
            NativePurity::Impure => "stateful",
        };
        return Some(format!(
            "**{word}** — native `{word}({arity} args) -> {result}`; {purity}; base cost {}.",
            spec.base_cost
        ));
    }
    if let Some(value) = lookup_math_constant(word) {
        return Some(format!(
            "**{word}** — reserved `float` mathematical constant (`{value}`)."
        ));
    }
    let text = match word {
        "import" => "Import a relative VS3 source, optionally under a module alias.",
        "export" => "Expose a function from its source module.",
        "function" => "Declare a function.",
        "scene" => "Declare a narrative scene.",
        "character" => "Declare a character.",
        "state" => "Declare persistent state bindings.",
        "let" => "Bind a local or global variable.",
        "return" => "Return from a function.",
        "if" | "else" | "while" => "Control flow.",
        "for" | "break" | "continue" => "Loop control.",
        "yield" => "Suspend a cooperative VS3 task and optionally receive a response.",
        "jump" => "Jump to a scene or label.",
        "choice" => "Present player choices.",
        _ => return None,
    };
    Some(format!("**{word}** — {text}"))
}

fn word_at(source: &str, line: u32, character: u32) -> Option<String> {
    let line_str = source.lines().nth(line as usize)?;
    if line_str.is_empty() {
        return None;
    }
    let chars: Vec<char> = line_str.chars().collect();
    let mut idx = (character as usize).min(chars.len().saturating_sub(1));
    if idx < chars.len() && !(chars[idx].is_ascii_alphanumeric() || chars[idx] == '_') {
        if idx > 0 && (chars[idx - 1].is_ascii_alphanumeric() || chars[idx - 1] == '_') {
            idx -= 1;
        } else {
            return None;
        }
    }
    let mut start = idx;
    while start > 0 && (chars[start - 1].is_ascii_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    let mut end = idx;
    while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    if start >= end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}

fn word_range_on_line(source: &str, line: u32, word: &str) -> Option<TextRange> {
    let line_str = source.lines().nth(line as usize)?;
    let start = line_str.find(word)?;
    Some(TextRange {
        line,
        character: start as u32,
        end_line: line,
        end_character: (start + word.len()) as u32,
    })
}

/// Collect simple keyword / identifier / literal semantic token ranges.
pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    const KEYWORDS: &[&str] = &[
        "import",
        "export",
        "function",
        "scene",
        "screen",
        "button",
        "character",
        "state",
        "choice",
        "jump",
        "label",
        "return",
        "let",
        "const",
        "if",
        "else",
        "while",
        "for",
        "in",
        "break",
        "continue",
        "yield",
        "background",
        "music",
        "show",
        "true",
        "false",
        "null",
        "int",
        "float",
        "bool",
        "string",
        "list",
        "map",
        "vec2",
        "vec3",
        "vec4",
        "mat3",
        "mat4",
        "quat",
        "rng",
    ];

    let mut tokens = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        // Strings
        let mut i = 0usize;
        let bytes = line.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
                tokens.push(SemanticToken {
                    range: TextRange {
                        line: line_idx as u32,
                        character: i as u32,
                        end_line: line_idx as u32,
                        end_character: bytes.len() as u32,
                    },
                    kind: SemanticTokenKind::Comment,
                    text: line[i..].to_string(),
                });
                break;
            }
            if bytes[i] == b'"' {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let text = line[start..i].to_string();
                tokens.push(SemanticToken {
                    range: TextRange {
                        line: line_idx as u32,
                        character: start as u32,
                        end_line: line_idx as u32,
                        end_character: i as u32,
                    },
                    kind: SemanticTokenKind::String,
                    text,
                });
                continue;
            }
            // Numbers
            if bytes[i].is_ascii_digit() {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let text = line[start..i].to_string();
                tokens.push(SemanticToken {
                    range: TextRange {
                        line: line_idx as u32,
                        character: start as u32,
                        end_line: line_idx as u32,
                        end_character: i as u32,
                    },
                    kind: SemanticTokenKind::Number,
                    text,
                });
                continue;
            }
            // Identifiers / keywords
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let text = &line[start..i];
                let kind = if KEYWORDS.contains(&text) {
                    SemanticTokenKind::Keyword
                } else {
                    SemanticTokenKind::Identifier
                };
                tokens.push(SemanticToken {
                    range: TextRange {
                        line: line_idx as u32,
                        character: start as u32,
                        end_line: line_idx as u32,
                        end_character: i as u32,
                    },
                    kind,
                    text: text.to_string(),
                });
                continue;
            }
            i += 1;
        }
    }

    // Upgrade function definition names when parse succeeds.
    if let Ok(parsed) = parse_file(source, None) {
        for item in &parsed.module.items {
            if let Item::Function { name, loc, .. } = item {
                let line = loc.line.saturating_sub(1);
                let character = loc.column.saturating_sub(1);
                for t in tokens.iter_mut() {
                    if t.range.line == line
                        && t.text == *name
                        && t.kind == SemanticTokenKind::Identifier
                    {
                        // function keyword is usually at loc; name may be nearby.
                        t.kind = SemanticTokenKind::Function;
                    }
                    let _ = character;
                }
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_scene_symbols() {
        let src = r#"
character aria { name: "Aria" }
scene main { aria "hi" }
function f() { return 1 }
"#;
        let a = analyze(src, Some("t.vel"));
        assert!(a
            .symbols
            .iter()
            .any(|s| s.kind == "scene" && s.name == "main"));
        assert!(a.symbols.iter().any(|s| s.kind == "function"));
        assert!(a.symbols.iter().any(|s| s.kind == "character"));
    }

    #[test]
    fn analyzes_declarative_screen_symbol_without_compiler_error() {
        let source = r#"
screen main_menu {
    button start { label: "START"; action: "start_run"; }
}
"#;
        let analysis = analyze(source, Some("menu.vel"));
        assert!(analysis
            .symbols
            .iter()
            .any(|symbol| symbol.kind == "screen" && symbol.name == "main_menu"));
        assert!(
            analysis.diagnostics.is_empty(),
            "{:?}",
            analysis.diagnostics
        );
    }

    #[test]
    fn completions_include_keywords() {
        let c = completions("", 0, 0);
        assert!(c.iter().any(|x| x == "scene"));
        assert!(c.iter().any(|x| x == "abs"));
    }

    #[test]
    fn goto_function() {
        let src = "function add(a, b) { return a + b }\n";
        let s = goto_definition(src, "add").unwrap();
        assert_eq!(s.kind, "function");
    }

    #[test]
    fn find_references_function() {
        let src = r#"
function add(a, b) {
    return a + b
}
function main() {
    return add(1, 2)
}
"#;
        let refs = find_references(src, "add");
        assert!(refs.len() >= 2, "refs={refs:?}");
    }

    #[test]
    fn rename_prepare_and_apply() {
        let src = "function add(a, b) { return a + b }\nlet x = add(1, 2)\n";
        let edits = rename_prepare(src, "add", "sum");
        assert!(!edits.is_empty());
        let out = apply_text_edits(src, edits);
        assert!(out.contains("function sum"));
        assert!(out.contains("sum(1, 2)"));
        assert!(!out.contains("add"));
    }

    #[test]
    fn references_ignore_strings_and_comments() {
        let source = r#"function add(a, b) { return a + b }
// add must not change here
let label = "add"
let value = add(1, 2)
"#;
        let edits = rename_prepare(source, "add", "sum");
        let output = apply_text_edits(source, edits);
        assert!(output.contains("function sum"));
        assert!(output.contains("sum(1, 2)"));
        assert!(output.contains("// add must not change here"));
        assert!(output.contains("\"add\""));
    }

    #[test]
    fn edition_three_uses_vs3_semantic_diagnostics_and_completions() {
        let source = "// @edition 3\nfunction f() { return missing }\n";
        let analysis = analyze(source, Some("logic.vel"));
        assert!(analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.source == "velvet-vs3"
                && diagnostic.message.contains("unknown name")));
        let completions = completions(source, 1, 0);
        assert!(completions.iter().any(|item| item == "yield"));
        assert!(completions.iter().any(|item| item == "import"));
        assert!(completions.iter().any(|item| item == "export"));
        assert!(completions.iter().any(|item| item == "map_keys"));
        assert!(completions.iter().any(|item| item == "vec3"));
        assert!(completions.iter().any(|item| item == "mat4_perspective"));
        assert!(completions.iter().any(|item| item == "perlin2"));
        assert!(completions.iter().any(|item| item == "correlation"));
        assert!(!completions.iter().any(|item| item == "scene"));
    }

    #[test]
    fn advanced_native_hover_uses_registry_metadata() {
        let hover = keyword_or_native_hover("mat_inverse").unwrap();
        assert!(hover.contains("matrix"), "{hover}");
        assert!(hover.contains("pure"), "{hover}");
        assert!(hover.contains("base cost"), "{hover}");
        let constant = keyword_or_native_hover("PI").unwrap();
        assert!(constant.contains("mathematical constant"), "{constant}");
    }

    #[test]
    fn hover_on_function() {
        let src = "function add(a, b) { return a + b }\n";
        // Find "add" position.
        let line = src.lines().next().unwrap();
        let col = line.find("add").unwrap() as u32;
        let h = hover(src, 0, col).unwrap();
        assert!(h.contents.contains("add"), "hover={h:?}");
        assert_eq!(h.kind.as_deref(), Some("function"));
    }

    #[test]
    fn semantic_tokens_keywords_and_idents() {
        let src = "export function add(a, b) { return a + 1 }\n";
        let toks = semantic_tokens(src);
        assert!(toks
            .iter()
            .any(|t| t.kind == SemanticTokenKind::Keyword && t.text == "function"));
        assert!(toks
            .iter()
            .any(|t| t.kind == SemanticTokenKind::Keyword && t.text == "export"));
        assert!(toks
            .iter()
            .any(|t| t.kind == SemanticTokenKind::Keyword && t.text == "return"));
        assert!(toks.iter().any(|t| t.text == "add"
            && matches!(
                t.kind,
                SemanticTokenKind::Identifier | SemanticTokenKind::Function
            )));
        assert!(toks
            .iter()
            .any(|t| t.kind == SemanticTokenKind::Number && t.text == "1"));
    }
}

/// VS2 IDE completions / hover.
pub mod vs2_ide;
