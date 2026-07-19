//! Parse a CSS + JS-lite hybrid (`.vcss` / Velvet Style Sheets).
//!
//! - **CSS side:** selectors, declarations, `@keyframes`
//! - **JS side:** `@script { let / fn / for / play / animate / on }`

use indexmap::IndexMap;
use thiserror::Error;

use crate::animation::{KeyframeStop, Keyframes};
use crate::script::{parse_script, ScriptError, ScriptModule};
use crate::value::{parse_value, StyleValue};

/// Parse error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StyleParseError {
    /// Message with optional line.
    #[error("style parse error at line {line}: {msg}")]
    AtLine {
        /// 1-based line.
        line: usize,
        /// Detail.
        msg: String,
    },
    /// JS-lite `@script` error.
    #[error("style script error: {0}")]
    Script(#[from] ScriptError),
}

/// One rule: selectors + declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleRule {
    /// Selectors (e.g. `.button`, `.button:selected`, `#start`).
    pub selectors: Vec<String>,
    /// Properties in source order.
    pub declarations: IndexMap<String, StyleValue>,
}

/// Parsed stylesheet — CSS look/motion + optional JS-lite script module.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stylesheet {
    /// Rules in cascade order.
    pub rules: Vec<StyleRule>,
    /// Named keyframe animations (replaces separate `.vanim`).
    pub keyframes: IndexMap<String, Keyframes>,
    /// JS-lite motion/orchestration (`@script` blocks, merged).
    pub script: ScriptModule,
    /// Optional source name.
    pub source: Option<String>,
}

impl Stylesheet {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another sheet (appended, later wins on same specificity ties).
    pub fn extend(&mut self, other: Stylesheet) {
        self.rules.extend(other.rules);
        for (k, v) in other.keyframes {
            self.keyframes.insert(k, v);
        }
        self.script.extend(other.script);
    }
}

/// Strip `/* … */` and `//` comments, preserving newlines for line numbers.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let b = source.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                if b[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(b.len());
        } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// Parse full stylesheet text (CSS rules + `@keyframes` + `@script`).
pub fn parse_stylesheet(source: &str) -> Result<Stylesheet, StyleParseError> {
    let source = strip_comments(source);
    let mut rules = Vec::new();
    let mut keyframes = IndexMap::new();
    let mut script = ScriptModule::default();
    let mut i = 0;
    let bytes = source.as_bytes();
    let mut line = 1usize;

    while i < bytes.len() {
        // skip whitespace
        while i < bytes.len() {
            if bytes[i] == b'\n' {
                line += 1;
                i += 1;
            } else if bytes[i].is_ascii_whitespace() {
                i += 1;
            } else {
                break;
            }
        }
        if i >= bytes.len() {
            break;
        }

        // selector / @rule header until {
        let sel_start = i;
        let sel_line = line;
        while i < bytes.len() && bytes[i] != b'{' {
            if bytes[i] == b'\n' {
                line += 1;
            }
            i += 1;
        }
        if i >= bytes.len() {
            return Err(StyleParseError::AtLine {
                line: sel_line,
                msg: "expected `{{` after selectors".into(),
            });
        }
        let sel_raw = source[sel_start..i].trim();
        i += 1; // {

        let decl_start = i;
        let mut depth = 1i32;
        // Inside @script, strings may contain `{`/`}` — track quotes for script only.
        let is_script = sel_raw == "@script" || sel_raw.starts_with("@script ");
        let mut in_str: Option<u8> = None;
        let mut escape = false;
        while i < bytes.len() && depth > 0 {
            let c = bytes[i];
            if is_script {
                if escape {
                    escape = false;
                } else if in_str.is_some() {
                    if c == b'\\' {
                        escape = true;
                    } else if Some(c) == in_str {
                        in_str = None;
                    }
                } else if c == b'"' || c == b'\'' {
                    in_str = Some(c);
                } else if c == b'{' {
                    depth += 1;
                } else if c == b'}' {
                    depth -= 1;
                }
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
            }
            if c == b'\n' {
                line += 1;
            }
            if depth > 0 {
                i += 1;
            }
        }
        if depth != 0 {
            return Err(StyleParseError::AtLine {
                line: sel_line,
                msg: "unclosed rule block".into(),
            });
        }
        let body = source[decl_start..i].trim();
        i += 1; // }

        if sel_raw == "@script" || sel_raw.starts_with("@script ") {
            let modu = parse_script(body)?;
            script.extend(modu);
            continue;
        }

        if let Some(rest) = sel_raw.strip_prefix("@keyframes") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(StyleParseError::AtLine {
                    line: sel_line,
                    msg: "@keyframes needs a name".into(),
                });
            }
            let kf = parse_keyframes_body(name, body, sel_line)?;
            keyframes.insert(name.to_string(), kf);
            continue;
        }

        let selectors: Vec<String> = sel_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if selectors.is_empty() {
            return Err(StyleParseError::AtLine {
                line: sel_line,
                msg: "empty selector".into(),
            });
        }

        let declarations = parse_declarations(body, sel_line)?;
        rules.push(StyleRule {
            selectors,
            declarations,
        });
    }

    Ok(Stylesheet {
        rules,
        keyframes,
        script,
        source: None,
    })
}

fn parse_keyframes_body(
    name: &str,
    body: &str,
    base_line: usize,
) -> Result<Keyframes, StyleParseError> {
    let mut stops = Vec::new();
    let mut i = 0;
    let b = body.as_bytes();
    let mut line = base_line;
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            if b[i] == b'\n' {
                line += 1;
            }
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let start = i;
        while i < b.len() && b[i] != b'{' {
            if b[i] == b'\n' {
                line += 1;
            }
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let offset_raw = body[start..i].trim();
        i += 1;
        let body_start = i;
        let mut depth = 1i32;
        while i < b.len() && depth > 0 {
            if b[i] == b'{' {
                depth += 1;
            } else if b[i] == b'}' {
                depth -= 1;
            } else if b[i] == b'\n' {
                line += 1;
            }
            if depth > 0 {
                i += 1;
            }
        }
        let stop_body = body[body_start..i].trim();
        i += 1;
        let offset = parse_keyframe_offset(offset_raw).ok_or_else(|| StyleParseError::AtLine {
            line,
            msg: format!("bad keyframe offset `{offset_raw}`"),
        })?;
        let props = parse_declarations(stop_body, line)?;
        stops.push(KeyframeStop { offset, props });
    }
    stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
    Ok(Keyframes {
        name: name.into(),
        stops,
    })
}

fn parse_keyframe_offset(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if s.eq_ignore_ascii_case("from") {
        return Some(0.0);
    }
    if s.eq_ignore_ascii_case("to") {
        return Some(1.0);
    }
    if let Some(p) = s.strip_suffix('%') {
        let v: f32 = p.trim().parse().ok()?;
        return Some((v / 100.0).clamp(0.0, 1.0));
    }
    let v: f32 = s.parse().ok()?;
    Some(v.clamp(0.0, 1.0))
}

fn parse_declarations(
    body: &str,
    base_line: usize,
) -> Result<IndexMap<String, StyleValue>, StyleParseError> {
    let mut map = IndexMap::new();
    let mut line = base_line;
    for part in body.split(';') {
        let line_inc = part.chars().filter(|c| *c == '\n').count();
        let part = part.trim();
        line += line_inc;
        if part.is_empty() {
            continue;
        }
        let (name, value) = part.split_once(':').ok_or_else(|| StyleParseError::AtLine {
            line,
            msg: format!("expected `property: value` in `{part}`"),
        })?;
        let name = name.trim();
        // Custom props keep --name; other names lowercased
        let name = if name.starts_with("--") {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        };
        if name.is_empty() {
            return Err(StyleParseError::AtLine {
                line,
                msg: "empty property name".into(),
            });
        }
        map.insert(name, parse_value(value));
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::StyleValue;

    #[test]
    fn parse_button_rules() {
        let src = r#"
        /* casino buttons */
        .button {
            background: #0a0c16;
            border-color: #b9964b;
            border-width: 1;
            color: #d2af64;
            height: 52;
            padding-x: 14;
        }
        .button:selected {
            background: #501e78;
            border-color: #ffdc96;
            glow: #dc50dc;
            color: #ffe496;
        }
        #start {
            icon: star;
        }
        "#;
        let sheet = parse_stylesheet(src).expect("parse");
        assert_eq!(sheet.rules.len(), 3);
        let bg = sheet.rules[0]
            .declarations
            .get("background")
            .unwrap()
            .as_color()
            .unwrap();
        assert_eq!(bg.r, 0x0a);
        let sel = &sheet.rules[1];
        assert!(sel.selectors.iter().any(|s| s == ".button:selected"));
        assert!(matches!(
            sheet.rules[2].declarations.get("icon"),
            Some(StyleValue::Keyword(_))
        ));
    }

    #[test]
    fn parse_css_plus_script() {
        let src = r#"
        .card.deal { animation: deal 0.32s cubic_out; }
        @keyframes deal {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @script {
          let stagger = 0.08;
          fn dealHand(n) {
            for (let i = 0; i < n; i = i + 1) {
              play("deal", { target: "card" + i, delay: i * stagger });
            }
          }
        }
        "#;
        let sheet = parse_stylesheet(src).expect("parse hybrid");
        assert_eq!(sheet.rules.len(), 1);
        assert!(sheet.keyframes.contains_key("deal"));
        assert!(sheet.script.functions.contains_key("dealHand"));
        assert!(sheet.script.globals.contains_key("stagger"));
    }

    #[test]
    fn casino_vcss_css_plus_js() {
        let src = include_str!("../../../demos/velvet-stakes/data/styles/casino.vcss");
        let sheet = parse_stylesheet(src).expect("casino.vcss");
        assert!(sheet.keyframes.contains_key("deal"));
        assert!(sheet.script.functions.contains_key("dealHand"));
        assert!(sheet.script.functions.contains_key("logoEnter"));
        assert!(!sheet.script.handlers.is_empty());
        let run = crate::call_style_fn(&sheet, "dealHand", &[crate::JsValue::num(5.0)])
            .expect("dealHand");
        assert_eq!(run.actions.len(), 5);
        assert_eq!(run.timelines.len(), 5);
    }
}
