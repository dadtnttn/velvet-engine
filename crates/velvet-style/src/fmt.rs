//! `.vcss` formatter: deterministic pretty-printing.
//!
//! Rules, `@media`, `@keyframes` and `@svg` blocks re-emit from the AST
//! (canonical spacing, values via [`display_value`]); `@script` bodies
//! pass through verbatim with brace-depth reindentation (the JS-lite AST
//! has no source-lossless printer yet). Output groups blocks by kind in
//! a fixed order and uses Unix newlines, so `fmt(fmt(x)) == fmt(x)`.

use crate::animation::KeyframeStop;
use crate::parse::{parse_stylesheet, StyleParseError, StyleRule};
use crate::value::{CalcExpr, Color, Gradient, StyleValue};

/// Format a stylesheet (validates first; errors on parse failure).
pub fn format_stylesheet(source: &str) -> Result<String, StyleParseError> {
    let sheet = parse_stylesheet(source)?;
    let mut out = String::new();
    for import in &sheet.imports {
        out.push_str(&format!("@import \"{import}\";\n"));
    }
    if !sheet.imports.is_empty() {
        out.push('\n');
    }
    for rule in &sheet.rules {
        push_rule(&mut out, rule, 0);
    }
    for block in &sheet.media {
        out.push_str(&format!("@media {} {{\n", block.query_text.trim()));
        for rule in &block.rules {
            push_rule(&mut out, rule, 1);
        }
        out.push_str("}\n\n");
    }
    for (_, kf) in &sheet.keyframes {
        push_keyframes(&mut out, kf);
    }
    for (_, svg) in &sheet.svgs {
        out.push_str(&format!("@svg {} {{\n", svg.name));
        out.push_str(&format!("  view: {} {};\n", svg.view_w, svg.view_h));
        out.push_str(&format!("  fill: {};\n", svg.fill));
        out.push_str(&format!("  stroke: {};\n", svg.stroke));
        out.push_str(&format!(
            "  stroke-width: {};\n",
            trim_num(svg.stroke_width)
        ));
        out.push_str(&format!("  path: {};\n", svg.path));
        out.push_str("}\n\n");
    }
    for body in extract_script_bodies(source) {
        out.push_str("@script {\n");
        out.push_str(&reindent_js(&body));
        out.push_str("}\n\n");
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn push_rule(out: &mut String, rule: &StyleRule, level: usize) {
    indent(out, level);
    out.push_str(&rule.selectors.join(", "));
    out.push_str(" {\n");
    for (k, v) in &rule.declarations {
        indent(out, level + 1);
        out.push_str(&format!("{k}: {};\n", display_value(v)));
    }
    indent(out, level);
    out.push_str("}\n");
    if level == 0 {
        out.push('\n');
    }
}

fn push_keyframes(out: &mut String, kf: &crate::animation::Keyframes) {
    out.push_str(&format!("@keyframes {} {{\n", kf.name));
    for stop in &kf.stops {
        push_keyframe_stop(out, stop);
    }
    out.push_str("}\n\n");
}

fn keyframe_label(stop: &KeyframeStop) -> String {
    if stop.offset <= 0.0 {
        "from".to_string()
    } else if stop.offset >= 1.0 {
        "to".to_string()
    } else {
        format!("{}%", trim_num(stop.offset * 100.0))
    }
}

fn push_keyframe_stop(out: &mut String, stop: &KeyframeStop) {
    let label = keyframe_label(stop);
    indent(out, 1);
    out.push_str(&label);
    out.push_str(" {\n");
    for (k, v) in &stop.props {
        indent(out, 2);
        out.push_str(&format!("{k}: {};\n", display_value(v)));
    }
    indent(out, 1);
    out.push_str("}\n");
}

/// Minified single-line stylesheet.
///
/// Structure is squeezed (no indentation/newlines, `,`-joined selectors,
/// trailing structure kept parseable); value internals reuse
/// [`display_value`] verbatim and `@script` bodies are comment-stripped +
/// whitespace-collapsed (the lexer skips both uniformly, so the token
/// stream — and semantics — are unchanged). Re-parses to the same AST:
/// `min(min(x)) == min(x)`.
pub fn format_stylesheet_min(source: &str) -> Result<String, StyleParseError> {
    let sheet = parse_stylesheet(source)?;
    let mut out = String::new();
    for import in &sheet.imports {
        out.push_str(&format!("@import \"{import}\";"));
    }
    for rule in &sheet.rules {
        push_rule_min(&mut out, rule);
    }
    for block in &sheet.media {
        out.push_str(&format!("@media {}{{", block.query_text.trim()));
        for rule in &block.rules {
            push_rule_min(&mut out, rule);
        }
        out.push('}');
    }
    for (_, kf) in &sheet.keyframes {
        out.push_str(&format!("@keyframes {}{{", kf.name));
        for stop in &kf.stops {
            out.push_str(&keyframe_label(stop));
            out.push('{');
            for (k, v) in &stop.props {
                out.push_str(&format!("{k}:{};", display_value(v)));
            }
            out.push('}');
        }
        out.push('}');
    }
    for (_, svg) in &sheet.svgs {
        out.push_str(&format!(
            "@svg {}{{view:{} {};fill:{};stroke:{};stroke-width:{};path:{};}}",
            svg.name,
            svg.view_w,
            svg.view_h,
            svg.fill,
            svg.stroke,
            trim_num(svg.stroke_width),
            svg.path
        ));
    }
    for body in extract_script_bodies(source) {
        out.push_str("@script{");
        out.push_str(&min_js(&body));
        out.push('}');
    }
    Ok(out)
}

fn push_rule_min(out: &mut String, rule: &StyleRule) {
    out.push_str(&rule.selectors.join(","));
    out.push('{');
    for (k, v) in &rule.declarations {
        out.push_str(&format!("{k}:{};", display_value(v)));
    }
    out.push('}');
}

/// Collapse JS-lite to minimal whitespace: comments become one space,
/// whitespace runs collapse to one, strings/templates pass through
/// byte-identical (nested `${…}` holes minify as code).
fn min_js(body: &str) -> String {
    #[derive(PartialEq)]
    enum Frame {
        /// Template literal (backtick), holes nest via `Hole`.
        Tpl,
        /// `${…}` hole with brace depth.
        Hole(u32),
    }
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    let mut stack: Vec<Frame> = Vec::new();
    // Inside a '/" string (None = code or template text).
    let mut str_quote: Option<u8> = None;
    let mut pending_space = false;
    // Copy the full char at `i` (multibyte-safe), return the advance.
    let push_char = |out: &mut String, i: &mut usize| {
        let ch = body[*i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        *i += ch.len_utf8();
    };
    // Whitespace/comments collapse to at most one space, and only between
    // two word chars (`return 5`, `a b` across a comment). Anything else
    // glues (`a+b`, `){`, `,b`): the lexer splits tokens identically
    // without the space, so semantics never change.
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';
    let push_space = |out: &mut String, pending_space: &mut bool, curr_word: bool| {
        let prev_word = out.as_bytes().last().is_some_and(|b| is_word(*b));
        if *pending_space && prev_word && curr_word {
            out.push(' ');
        }
        *pending_space = false;
    };
    while i < bytes.len() {
        let c = bytes[i];
        // Plain '/" strings: verbatim until the unescaped quote.
        if let Some(q) = str_quote {
            push_char(&mut out, &mut i);
            if c == b'\\' && i < bytes.len() {
                push_char(&mut out, &mut i);
            } else if c == q {
                str_quote = None;
            }
            continue;
        }
        // Template text: verbatim, but backtick ends and ${ opens a hole.
        if stack.last() == Some(&Frame::Tpl) {
            if c == b'\\' {
                push_char(&mut out, &mut i);
                if i < bytes.len() {
                    push_char(&mut out, &mut i);
                }
                continue;
            }
            if c == b'`' {
                out.push(c as char);
                i += 1;
                stack.pop();
                continue;
            }
            if c == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                out.push_str("${");
                i += 2;
                stack.push(Frame::Hole(0));
                continue;
            }
            push_char(&mut out, &mut i);
            continue;
        }
        match c {
            b'"' | b'\'' => {
                push_space(&mut out, &mut pending_space, false);
                out.push(c as char);
                i += 1;
                str_quote = Some(c);
            }
            b'`' => {
                push_space(&mut out, &mut pending_space, false);
                out.push(c as char);
                i += 1;
                stack.push(Frame::Tpl);
            }
            b'{' => {
                if let Some(Frame::Hole(d)) = stack.last_mut() {
                    *d += 1;
                }
                push_space(&mut out, &mut pending_space, false);
                out.push(c as char);
                i += 1;
            }
            b'}' => {
                let mut close_hole = false;
                if let Some(Frame::Hole(d)) = stack.last_mut() {
                    if *d == 0 {
                        close_hole = true;
                    } else {
                        *d -= 1;
                    }
                }
                if close_hole {
                    stack.pop();
                }
                push_space(&mut out, &mut pending_space, false);
                out.push(c as char);
                i += 1;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // Line comment → one space (never merge tokens).
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                pending_space = true;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                pending_space = true;
            }
            _ if c.is_ascii_whitespace() => {
                pending_space = true;
                i += 1;
            }
            // Closers never need a preceding space (`{a: 1}` → `{a:1}`).
            b':' | b';' | b',' | b')' | b']' => {
                pending_space = false;
                out.push(c as char);
                i += 1;
            }
            _ => {
                push_space(&mut out, &mut pending_space, is_word(c));
                push_char(&mut out, &mut i);
            }
        }
    }
    out
}

fn trim_num(v: f32) -> String {
    if v.fract() == 0.0 && v.abs() < 1e12 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Canonical single-line display for a value.
pub fn display_value(value: &StyleValue) -> String {
    match value {
        StyleValue::Color(c) => display_color(c),
        StyleValue::Length(v) => format!("{}px", trim_num(*v)),
        StyleValue::Number(v) => trim_num(*v),
        StyleValue::Percent(v) => format!("{}%", trim_num(*v)),
        StyleValue::Time(v) => format!("{}s", trim_num(*v)),
        StyleValue::Keyword(s) | StyleValue::String(s) => s.clone(),
        StyleValue::Var(name) => format!("var({name})"),
        StyleValue::VarFallback { name, fallback } => {
            format!("var({name}, {})", display_value(fallback))
        }
        StyleValue::Url(path) => format!("url(\"{path}\")"),
        StyleValue::SvgRef(name) => format!("svg({name})"),
        StyleValue::Calc(expr) => display_calc(expr),
        StyleValue::Gradient(g) => display_gradient(g),
        StyleValue::ColorMix { a, wa, b, wb } => format!(
            "color-mix(in srgb, {} {}%, {} {}%)",
            display_value(a),
            trim_num(*wa),
            display_value(b),
            trim_num(*wb)
        ),
        StyleValue::Alpha { color, amount } => {
            format!("alpha({}, {})", display_value(color), trim_num(*amount))
        }
        StyleValue::AutoContrast { base, dark, light } => format!(
            "auto-contrast({}, {}, {})",
            display_value(base),
            display_value(dark),
            display_value(light)
        ),
        StyleValue::Shadow(layers) => layers
            .iter()
            .map(|s| {
                let base = format!(
                    "{} {} {} {} {}",
                    trim_num(s.offset_x),
                    trim_num(s.offset_y),
                    trim_num(s.blur),
                    trim_num(s.spread),
                    display_value(&s.color)
                );
                if s.inset {
                    format!("{base} inset")
                } else {
                    base
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn display_color(c: &Color) -> String {
    if (c.a - 1.0).abs() < 1e-6 {
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, trim_num(c.a))
    }
}

fn display_calc(expr: &CalcExpr) -> String {
    fn num(v: &StyleValue) -> String {
        display_value(v)
    }
    match expr {
        CalcExpr::Lit(v) => num(v),
        CalcExpr::Add(a, b) => format!("calc({} + {})", display_calc(a), display_calc(b)),
        CalcExpr::Sub(a, b) => format!("calc({} - {})", display_calc(a), display_calc(b)),
        CalcExpr::Mul(a, b) => format!("calc({} * {})", display_calc(a), display_calc(b)),
        CalcExpr::Div(a, b) => format!("calc({} / {})", display_calc(a), display_calc(b)),
        CalcExpr::Min(args) => format!(
            "min({})",
            args.iter().map(display_calc).collect::<Vec<_>>().join(", ")
        ),
        CalcExpr::Max(args) => format!(
            "max({})",
            args.iter().map(display_calc).collect::<Vec<_>>().join(", ")
        ),
        CalcExpr::Clamp(lo, val, hi) => format!(
            "clamp({}, {}, {})",
            display_calc(lo),
            display_calc(val),
            display_calc(hi)
        ),
        CalcExpr::Var { name, fallback } => match fallback {
            Some(fb) => format!("var({name}, {})", display_calc(fb)),
            None => format!("var({name})"),
        },
    }
}

fn display_gradient(g: &Gradient) -> String {
    let stops = g
        .stops
        .iter()
        .map(|s| match s.position {
            Some(p) => format!("{} {}%", display_value(&s.color), trim_num(p * 100.0)),
            None => display_value(&s.color),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if g.radial {
        format!(
            "radial-gradient(circle at {}% {}%, {stops})",
            trim_num(g.center[0] * 100.0),
            trim_num(g.center[1] * 100.0)
        )
    } else {
        format!("linear-gradient({}deg, {stops})", trim_num(g.angle_deg))
    }
}

/// Raw `@script` bodies in source order (brace-matched, string-aware
/// so `@script` inside a JS string never starts a false block).
fn extract_script_bodies(source: &str) -> Vec<String> {
    let mut bodies = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut in_str: Option<u8> = None;
    let mut escape = false;
    while i < bytes.len() {
        let c = bytes[i];
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        if in_str.is_some() {
            if c == b'\\' {
                escape = true;
            } else if Some(c) == in_str {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            in_str = Some(c);
            i += 1;
            continue;
        }
        // Byte-level prefix check: `i` may sit inside a multibyte char
        // while this byte-wise scan advances, and slicing would panic.
        if bytes[i..].starts_with(b"@script")
            && (i + 7 >= bytes.len() || !bytes[i + 7].is_ascii_alphanumeric())
        {
            let mut j = i + 7;
            while j < bytes.len() && bytes[j] != b'{' {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            j += 1;
            let start = j;
            // Scan modes: plain strings, template text (braces ignored
            // except `${`, which opens a hole), and hole code (braces
            // count, nested templates allowed). Comments never contribute
            // braces either.
            #[derive(PartialEq)]
            enum Scan {
                Str(u8),
                Tpl,
                Hole(u32),
            }
            let mut depth = 1i32;
            let mut stack: Vec<Scan> = Vec::new();
            let mut escape = false;
            while j < bytes.len() && depth > 0 {
                let c = bytes[j];
                if escape {
                    escape = false;
                } else if matches!(stack.last(), Some(Scan::Str(_))) {
                    if c == b'\\' {
                        escape = true;
                    } else if matches!(stack.last(), Some(Scan::Str(q)) if *q == c) {
                        stack.pop();
                    }
                } else if stack.last() == Some(&Scan::Tpl) {
                    if c == b'\\' {
                        escape = true;
                    } else if c == b'`' {
                        stack.pop();
                    } else if c == b'$' && j + 1 < bytes.len() && bytes[j + 1] == b'{' {
                        stack.push(Scan::Hole(0));
                        depth += 1;
                        j += 1;
                    }
                } else if c == b'"' || c == b'\'' {
                    stack.push(Scan::Str(c));
                } else if c == b'`' {
                    stack.push(Scan::Tpl);
                } else if c == b'/' && j + 1 < bytes.len() && bytes[j + 1] == b'/' {
                    while j < bytes.len() && bytes[j] != b'\n' {
                        j += 1;
                    }
                    continue;
                } else if c == b'/' && j + 1 < bytes.len() && bytes[j + 1] == b'*' {
                    j += 2;
                    while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                        j += 1;
                    }
                    j = (j + 2).min(bytes.len());
                    continue;
                } else if c == b'{' {
                    depth += 1;
                    if let Some(Scan::Hole(d)) = stack.last_mut() {
                        *d += 1;
                    }
                } else if c == b'}' {
                    depth -= 1;
                    let mut close_hole = false;
                    if let Some(Scan::Hole(d)) = stack.last_mut() {
                        if *d == 0 {
                            close_hole = true;
                        } else {
                            *d -= 1;
                        }
                    }
                    if close_hole {
                        stack.pop();
                    }
                }
                j += 1;
            }
            bodies.push(source[start..j.saturating_sub(1)].to_string());
            i = j;
        } else {
            i += 1;
        }
    }
    bodies
}

/// Reindent JS-lite lines by brace depth (comments preserved verbatim).
fn reindent_js(body: &str) -> String {
    let mut out = String::new();
    let mut depth = 1usize;
    for raw_line in body.lines() {
        let mut line = raw_line.trim();
        // Strip one trailing \r for CRLF sources (output is Unix).
        line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        // A leading close brace dedents first (naive: counts `}` outside
        // strings/comments at line start only — stable for JS-lite).
        let code = strip_line_comment(line);
        let closes = code.chars().take_while(|c| *c == '}').count();
        depth = depth.saturating_sub(closes.min(depth.saturating_sub(1)));
        for _ in 0..depth {
            out.push_str("  ");
        }
        out.push_str(line);
        out.push('\n');
        let opens = count_braces(&code[closes.min(code.len())..]);
        depth += opens;
        void_stable_indent(&mut depth);
    }
    out
}

/// Keep depth ≥ 1 while inside the script block.
fn void_stable_indent(depth: &mut usize) {
    if *depth < 1 {
        *depth = 1;
    }
}

/// `{` minus `}` outside strings and `//` comments.
fn count_braces(code: &str) -> usize {
    let code = strip_line_comment(code);
    let (mut opens, mut closes, mut in_str, mut escape) = (0usize, 0usize, None, false);
    for c in code.bytes() {
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
            opens += 1;
        } else if c == b'}' {
            closes += 1;
        }
    }
    opens.saturating_sub(closes)
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_str: Option<u8> = None;
    let mut escape = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
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
        } else if c == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            return &line[..i];
        }
        i += 1;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        format_stylesheet(src).unwrap()
    }

    #[test]
    fn formats_rules_and_keyframes() {
        let pretty =
            fmt(".a{color:red;background:#000}\n@keyframes k{from{opacity:0}to{opacity:1}}\n");
        assert!(pretty.contains(".a {\n  color: #ff0000;\n"), "{pretty:?}");
        assert!(pretty.contains("@keyframes k {\n  from {\n"), "{pretty:?}");
        // Idempotent.
        assert_eq!(fmt(&pretty), pretty);
    }

    #[test]
    fn fmt_survives_utf8_comments_and_values() {
        // The pre-minify scanner steps byte-wise; slicing must never split
        // a multibyte char (em dashes, accents, CJK, emoji…).
        let src = "/* Mi Novela — hoja de Zoé 日本語 🎲 */\n.a { content: \"¡hola — ñ!\"; }\n";
        let pretty = fmt(src);
        assert!(pretty.contains("¡hola — ñ!"), "{pretty:?}");
        let min = format_stylesheet_min(src).unwrap();
        assert!(!min.contains('\n'), "{min:?}");
        assert!(min.contains("¡hola — ñ!"), "{min:?}");
        assert_eq!(format_stylesheet_min(&min).unwrap(), min);
        assert_eq!(format_stylesheet(&min).unwrap(), pretty);
    }

    #[test]
    fn min_squeezes_structure_but_keeps_ast() {
        let src = ".a {\n  color: red;\n  width: calc(8px + 4px);\n}\n\
                   @media (prefers-reduced-motion) {\n  .b { opacity: 0.5; }\n}\n\
                   @keyframes k {\n  from { opacity: 0; }\n  to { opacity: 1; }\n}\n";
        let min = format_stylesheet_min(src).unwrap();
        assert!(!min.contains('\n'), "{min:?}");
        assert_eq!(
            min,
            ".a{color:#ff0000;width:12px;}@media (prefers-reduced-motion){.b{opacity:0.5;}}\
             @keyframes k{from{opacity:0;}to{opacity:1;}}"
        );
        // Stable, and pretty-prints back to the canonical pretty form.
        assert_eq!(format_stylesheet_min(&min).unwrap(), min);
        assert_eq!(
            format_stylesheet(&min).unwrap(),
            format_stylesheet(src).unwrap()
        );
    }

    #[test]
    fn min_keeps_script_semantics() {
        let src = "@script {\n  // deal the hand\n  fn go(n) {\n    let msg = `hola ${n + 1} //ño`; /* multi\n      line */\n    return \"a//b\" + 'c' + msg;\n  }\n}\n";
        let min = format_stylesheet_min(src).unwrap();
        assert!(!min.contains("// deal"), "{min:?}");
        assert!(!min.contains("multi"), "{min:?}");
        // Strings, templates (incl. // and UTF-8) survive byte-identical.
        assert!(min.contains("`hola ${n+1} //ño`"), "{min:?}");
        assert!(min.contains("\"a//b\""), "{min:?}");
        // Pretty and minified bodies run to the same value.
        let bodies = extract_script_bodies(src);
        let min_bodies = extract_script_bodies(&min);
        assert_eq!(bodies.len(), 1);
        assert_eq!(min_bodies.len(), 1);
        let pretty_run = crate::script::eval_script_fn(
            &bodies[0],
            None,
            "go",
            &[crate::script::JsValue::num(2.0)],
        )
        .unwrap();
        let min_run = crate::script::eval_script_fn(
            &min_bodies[0],
            None,
            "go",
            &[crate::script::JsValue::num(2.0)],
        )
        .unwrap();
        assert_eq!(pretty_run.value, min_run.value);
        assert_eq!(min_run.value.as_string(), "a//bchola 3 //ño");
    }

    #[test]
    fn formats_media_and_values() {
        let pretty = fmt(
            "@media (prefers-reduced-motion){.b{width:calc(8px + 4px);background:color-mix(in srgb, red, blue);}}\n",
        );
        assert!(
            pretty.contains("@media (prefers-reduced-motion) {"),
            "{pretty:?}"
        );
        assert!(pretty.contains("width: 12px;"), "{pretty:?}");
        assert!(pretty.contains("background: #800080;"), "{pretty:?}");
        assert_eq!(fmt(&pretty), pretty);
    }

    #[test]
    fn script_body_survives_verbatim() {
        let pretty = fmt("@script {\nfn go(){return 1;}\n}\n.a { color: red; }\n");
        assert!(pretty.contains("fn go(){return 1;}"), "{pretty:?}");
        assert_eq!(fmt(&pretty), pretty);
    }

    #[test]
    fn display_roundtrips_values() {
        assert_eq!(display_value(&crate::value::parse_value("0.35s")), "0.35s");
        assert_eq!(display_value(&crate::value::parse_value("50%")), "50%");
        assert_eq!(
            display_value(&crate::value::parse_value("var(--x, red)")),
            "var(--x, #ff0000)"
        );
    }
}
