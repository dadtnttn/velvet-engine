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
    /// Relative `@import` paths (order preserved).
    pub imports: Vec<String>,
    /// Named SVG snippets from `@svg name { … }`.
    pub svgs: IndexMap<String, SvgDef>,
    /// Optional source name.
    pub source: Option<String>,
}

/// Inline SVG definition authored in `.vcss`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SvgDef {
    /// Name.
    pub name: String,
    /// viewBox width (default 64).
    pub view_w: u32,
    /// viewBox height (default 64).
    pub view_h: u32,
    /// Fill color string (#rrggbb or keyword).
    pub fill: String,
    /// Stroke color.
    pub stroke: String,
    /// Stroke width.
    pub stroke_width: f32,
    /// Path `d` attribute.
    pub path: String,
}

impl SvgDef {
    /// Emit SVG XML document.
    pub fn to_svg_xml(&self) -> String {
        velvet_image::build_svg_document(
            self.view_w.max(1),
            self.view_h.max(1),
            if self.fill.is_empty() {
                "#ebc878"
            } else {
                &self.fill
            },
            if self.stroke.is_empty() {
                "none"
            } else {
                &self.stroke
            },
            self.stroke_width,
            &self.path,
        )
    }
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
        for (k, v) in other.svgs {
            self.svgs.insert(k, v);
        }
        // imports already resolved when using parse_stylesheet_with_imports
    }
}

/// Strip `/* … */` and `//` comments, preserving newlines for line numbers.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut quote = None;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if let Some(end_quote) = quote {
            out.push(ch);
            if ch == '\\' {
                escaped = true;
            } else if ch == end_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            out.push(ch);
            continue;
        }
        if ch != '/' {
            out.push(ch);
            continue;
        }
        match chars.peek().copied() {
            Some('*') => {
                chars.next();
                out.push(' ');
                let mut previous = '\0';
                for comment_ch in chars.by_ref() {
                    if comment_ch == '\n' {
                        out.push('\n');
                    }
                    if previous == '*' && comment_ch == '/' {
                        break;
                    }
                    previous = comment_ch;
                }
            }
            Some('/') => {
                chars.next();
                out.push(' ');
                for comment_ch in chars.by_ref() {
                    if comment_ch == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Parse full stylesheet text (CSS rules + `@keyframes` + `@script`).
///
/// `@import "path";` lines are recorded on the sheet as [`Stylesheet::imports`]
/// (not auto-loaded — use [`parse_stylesheet_with_imports`]).
pub fn parse_stylesheet(source: &str) -> Result<Stylesheet, StyleParseError> {
    let source = strip_comments(source);
    let mut rules = Vec::new();
    let mut keyframes = IndexMap::new();
    let mut script = ScriptModule::default();
    let mut imports = Vec::new();
    let mut svgs: IndexMap<String, SvgDef> = IndexMap::new();
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

        // @import "path";  (no block)
        if source[i..].starts_with("@import") {
            let imp_line = line;
            let start = i;
            while i < bytes.len() && bytes[i] != b';' {
                if bytes[i] == b'\n' {
                    line += 1;
                }
                i += 1;
            }
            if i >= bytes.len() {
                return Err(StyleParseError::AtLine {
                    line: imp_line,
                    msg: "@import needs trailing `;`".into(),
                });
            }
            let stmt = source[start..i].trim();
            i += 1; // ;
            let path = extract_import_path(stmt).ok_or_else(|| StyleParseError::AtLine {
                line: imp_line,
                msg: format!("bad @import `{stmt}`"),
            })?;
            imports.push(path);
            continue;
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
        // Strings in CSS values, SVG paths, and scripts may contain braces.
        let mut in_str: Option<u8> = None;
        let mut escape = false;
        while i < bytes.len() && depth > 0 {
            let c = bytes[i];
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

        if let Some(rest) = sel_raw.strip_prefix("@svg") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(StyleParseError::AtLine {
                    line: sel_line,
                    msg: "@svg needs a name".into(),
                });
            }
            let def = parse_svg_def(name, body);
            svgs.insert(name.to_string(), def);
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
        imports,
        svgs,
        source: None,
    })
}

fn parse_svg_def(name: &str, body: &str) -> SvgDef {
    let mut def = SvgDef {
        name: name.into(),
        view_w: 64,
        view_h: 64,
        fill: "#ebc878".into(),
        stroke: "none".into(),
        stroke_width: 0.0,
        path: String::new(),
    };
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, v)) = part.split_once(':') else {
            continue;
        };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
        match k.as_str() {
            "viewbox" => {
                let nums: Vec<f32> = v
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                if nums.len() >= 4 {
                    def.view_w = nums[2].max(1.0) as u32;
                    def.view_h = nums[3].max(1.0) as u32;
                }
            }
            "fill" => def.fill = v.to_string(),
            "stroke" => def.stroke = v.to_string(),
            "stroke-width" => def.stroke_width = v.parse().unwrap_or(0.0),
            "path" | "d" => def.path = v.to_string(),
            "width" => def.view_w = v.parse().unwrap_or(64),
            "height" => def.view_h = v.parse().unwrap_or(64),
            _ => {}
        }
    }
    def
}

fn extract_import_path(stmt: &str) -> Option<String> {
    let rest = stmt.strip_prefix("@import")?.trim();
    if let Some(q) = rest.strip_prefix('"').and_then(|s| s.split('"').next()) {
        return Some(q.to_string());
    }
    if let Some(q) = rest.strip_prefix('\'').and_then(|s| s.split('\'').next()) {
        return Some(q.to_string());
    }
    if let Some(q) = rest.strip_prefix("url(").and_then(|s| s.strip_suffix(')')) {
        let q = q.trim().trim_matches(|c| c == '"' || c == '\'');
        return Some(q.to_string());
    }
    None
}

/// Parse a file and recursively load `@import` relatives against `base_dir`.
pub fn parse_stylesheet_with_imports(
    source: &str,
    base_dir: &std::path::Path,
) -> Result<Stylesheet, StyleParseError> {
    parse_with_imports_inner(source, base_dir, &mut Vec::new(), 0)
}

fn parse_with_imports_inner(
    source: &str,
    base_dir: &std::path::Path,
    stack: &mut Vec<std::path::PathBuf>,
    depth: u8,
) -> Result<Stylesheet, StyleParseError> {
    if depth > 16 {
        return Err(StyleParseError::AtLine {
            line: 1,
            msg: "@import nesting too deep".into(),
        });
    }
    let mut sheet = parse_stylesheet(source)?;
    let import_list = std::mem::take(&mut sheet.imports);
    let mut merged = Stylesheet::new();
    for rel in import_list {
        let path = base_dir.join(&rel);
        let canon = path.canonicalize().unwrap_or(path.clone());
        if stack.iter().any(|p| p == &canon) {
            continue; // cycle
        }
        let text = std::fs::read_to_string(&path).map_err(|e| StyleParseError::AtLine {
            line: 1,
            msg: format!("@import `{}`: {e}", path.display()),
        })?;
        let parent = path.parent().unwrap_or(base_dir);
        stack.push(canon);
        let child = parse_with_imports_inner(&text, parent, stack, depth + 1)?;
        stack.pop();
        merged.extend(child);
    }
    merged.extend(sheet);
    Ok(merged)
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
        let mut in_str: Option<u8> = None;
        let mut escaped = false;
        while i < b.len() && depth > 0 {
            let ch = b[i];
            if escaped {
                escaped = false;
            } else if in_str.is_some() {
                if ch == b'\\' {
                    escaped = true;
                } else if Some(ch) == in_str {
                    in_str = None;
                }
            } else if ch == b'"' || ch == b'\'' {
                in_str = Some(ch);
            } else if ch == b'{' {
                depth += 1;
            } else if ch == b'}' {
                depth -= 1;
            }
            if ch == b'\n' {
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
    let mut start = 0usize;
    let mut part_line = base_line;
    let mut line = base_line;
    let mut quote = None;
    let mut escaped = false;
    let mut nesting = 0u32;

    for (index, ch) in body.char_indices() {
        if ch == '\n' {
            line += 1;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(end_quote) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == end_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' | '[' | '{' => nesting += 1,
            ')' | ']' | '}' => nesting = nesting.saturating_sub(1),
            ';' if nesting == 0 => {
                parse_declaration_part(&body[start..index], part_line, &mut map)?;
                start = index + ch.len_utf8();
                part_line = line;
            }
            _ => {}
        }
    }
    parse_declaration_part(&body[start..], part_line, &mut map)?;
    Ok(map)
}

fn parse_declaration_part(
    raw: &str,
    base_line: usize,
    map: &mut IndexMap<String, StyleValue>,
) -> Result<(), StyleParseError> {
    let leading = raw.len() - raw.trim_start().len();
    let line = base_line + raw[..leading].chars().filter(|ch| *ch == '\n').count();
    let part = raw.trim();
    if part.is_empty() {
        return Ok(());
    }
    let (name, value) = part
        .split_once(':')
        .ok_or_else(|| StyleParseError::AtLine {
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
    Ok(())
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
    fn preserves_utf8_and_comment_markers_inside_strings() {
        let sheet = parse_stylesheet(
            r#"
            .leyenda {
              content: "Niño // listo; cierre } /* literal */";
              font-family: 'Señal';
            }
            "#,
        )
        .unwrap();
        let declarations = &sheet.rules[0].declarations;
        assert_eq!(
            declarations.get("content"),
            Some(&StyleValue::String(
                "Niño // listo; cierre } /* literal */".into()
            ))
        );
        assert_eq!(
            declarations.get("font-family"),
            Some(&StyleValue::String("Señal".into()))
        );
    }

    #[test]
    fn declarations_allow_semicolons_inside_strings() {
        let sheet =
            parse_stylesheet(r#".asset { source: "https://cdn.example/a;b.png"; color: #fff; }"#)
                .unwrap();
        let declarations = &sheet.rules[0].declarations;
        assert_eq!(
            declarations.get("source"),
            Some(&StyleValue::String("https://cdn.example/a;b.png".into()))
        );
        assert!(declarations.get("color").unwrap().as_color().is_some());
    }

    #[test]
    fn script_strings_preserve_urls_and_semicolons() {
        let sheet = parse_stylesheet(
            r#"
            @script {
              let endpoint = "https://api.example/v1;a=b";
              fn readEndpoint() { return endpoint; }
            }
            "#,
        )
        .unwrap();
        let run = crate::call_style_fn(&sheet, "readEndpoint", &[]).unwrap();
        assert_eq!(
            run.value,
            crate::script::JsValue::String("https://api.example/v1;a=b".into())
        );
    }

    #[test]
    fn parse_import_statement() {
        let sheet = parse_stylesheet(
            r#"
            @import "base.vcss";
            .x { color: #fff; }
            "#,
        )
        .unwrap();
        assert_eq!(sheet.imports, vec!["base.vcss".to_string()]);
        assert_eq!(sheet.rules.len(), 1);
    }

    #[test]
    fn parse_svg_block_and_url() {
        let sheet = parse_stylesheet(
            r#"
            @svg badge {
              viewBox: 0 0 64 64;
              fill: #ebc878;
              path: "M0,32 L32,0 L64,32 L32,64 Z";
            }
            .chip {
              background-image: svg(badge);
              width: 64;
              height: 64;
            }
            .panel {
              background-image: url("ui/bg.png");
            }
            "#,
        )
        .unwrap();
        assert!(sheet.svgs.contains_key("badge"));
        assert!(!sheet.svgs["badge"].path.is_empty());
        let chip = sheet
            .rules
            .iter()
            .find(|r| r.selectors[0] == ".chip")
            .unwrap();
        assert!(matches!(
            chip.declarations.get("background-image"),
            Some(StyleValue::SvgRef(n)) if n == "badge"
        ));
        let panel = sheet
            .rules
            .iter()
            .find(|r| r.selectors[0] == ".panel")
            .unwrap();
        assert!(matches!(
            panel.declarations.get("background-image"),
            Some(StyleValue::Url(u)) if u == "ui/bg.png"
        ));
        let xml = sheet.svgs["badge"].to_svg_xml();
        let img = velvet_image::rasterize_simple_svg(&xml, 32, 32).unwrap();
        assert!(img.pixels.iter().any(|&b| b != 0));
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
