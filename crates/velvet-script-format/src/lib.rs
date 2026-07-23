//! # velvet-script-format
//!
//! Deterministic formatter for Velvet Script (pretty-print from AST).

#![deny(missing_docs)]

use velvet_script_ast::{BinOp, Expr, Item, Module, Stmt, UnaryOp};
use velvet_script_parser::{parse_file, ParseError};

/// Format source; returns pretty string or parse error.
pub fn format_source(source: &str) -> Result<String, ParseError> {
    let parsed = parse_file(source, None)?;
    let mut comments = CommentCursor::from_source(source);
    Ok(format_module_with_comments(&parsed.module, &mut comments))
}

/// Format a module AST.
///
/// Top-level items are separated by a blank line. Blocks are indented with
/// four spaces. Binary operators are padded with spaces.
pub fn format_module(module: &Module) -> String {
    format_module_with_comments(module, &mut CommentCursor::default())
}

fn format_module_with_comments(module: &Module, comments: &mut CommentCursor) -> String {
    let mut out = String::new();
    for (i, item) in module.items.iter().enumerate() {
        if i > 0 {
            // Blank line between top-level items.
            out.push('\n');
        }
        format_item(item, &mut out, 0, comments);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    comments.flush(&mut out);
    out
}

#[derive(Debug, Default)]
struct CommentCursor {
    comments: Vec<LineComment>,
    next: usize,
}

#[derive(Debug)]
struct LineComment {
    line: u32,
    indent: String,
    text: String,
}

impl CommentCursor {
    fn from_source(source: &str) -> Self {
        let comments = source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                comment_start(line).map(|start| LineComment {
                    line: index as u32 + 1,
                    indent: line[..start]
                        .chars()
                        .take_while(|character| character.is_whitespace())
                        .collect(),
                    text: line[start..].trim_end().to_string(),
                })
            })
            .collect();
        Self { comments, next: 0 }
    }

    fn emit_before(&mut self, line: u32, out: &mut String) {
        while self
            .comments
            .get(self.next)
            .is_some_and(|comment| comment.line < line)
        {
            self.emit_next(out);
        }
    }

    fn flush(&mut self, out: &mut String) {
        while self.next < self.comments.len() {
            self.emit_next(out);
        }
    }

    fn emit_next(&mut self, out: &mut String) {
        let comment = &self.comments[self.next];
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&comment.indent);
        out.push_str(&comment.text);
        out.push('\n');
        self.next += 1;
    }
}

fn comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quote.is_some() {
            escaped = true;
        } else if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(byte);
            }
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'/') && quote.is_none() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

fn format_item(item: &Item, out: &mut String, level: usize, comments: &mut CommentCursor) {
    comments.emit_before(item_line(item), out);
    match item {
        Item::Function {
            name, params, body, ..
        } => {
            indent(out, level);
            out.push_str("function ");
            out.push_str(name);
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                if let Some(t) = &p.ty {
                    out.push_str(": ");
                    out.push_str(t);
                }
            }
            out.push_str(") {\n");
            for s in body {
                format_stmt(s, out, level + 1, comments);
            }
            indent(out, level);
            out.push('}');
        }
        Item::Character { name, fields, .. } => {
            indent(out, level);
            out.push_str("character ");
            out.push_str(name);
            out.push_str(" {\n");
            for (k, v) in fields {
                comments.emit_before(v.loc().line, out);
                indent(out, level + 1);
                out.push_str(k);
                out.push_str(": ");
                format_expr(v, out, 0);
                out.push('\n');
            }
            indent(out, level);
            out.push('}');
        }
        Item::State { bindings, .. } => {
            indent(out, level);
            out.push_str("state {\n");
            for b in bindings {
                comments.emit_before(b.loc.line, out);
                indent(out, level + 1);
                out.push_str(&b.name);
                if let Some(t) = &b.ty {
                    out.push_str(": ");
                    out.push_str(t);
                }
                out.push_str(" = ");
                format_expr(&b.init, out, 0);
                out.push('\n');
            }
            indent(out, level);
            out.push('}');
        }
        Item::Scene { name, body, .. } => {
            indent(out, level);
            out.push_str("scene ");
            out.push_str(name);
            out.push_str(" {\n");
            for s in body {
                format_stmt(s, out, level + 1, comments);
            }
            indent(out, level);
            out.push('}');
        }
        Item::Screen {
            name,
            properties,
            buttons,
            ..
        } => {
            indent(out, level);
            out.push_str("screen ");
            out.push_str(name);
            out.push_str(" {\n");
            for property in properties {
                comments.emit_before(property.loc.line, out);
                indent(out, level + 1);
                out.push_str(&property.name);
                out.push_str(": ");
                format_expr(&property.value, out, 0);
                out.push('\n');
            }
            if !properties.is_empty() && !buttons.is_empty() {
                out.push('\n');
            }
            for (index, button) in buttons.iter().enumerate() {
                comments.emit_before(button.loc.line, out);
                indent(out, level + 1);
                out.push_str("button ");
                out.push_str(&button.id);
                out.push_str(" {\n");
                for property in &button.properties {
                    comments.emit_before(property.loc.line, out);
                    indent(out, level + 2);
                    out.push_str(&property.name);
                    out.push_str(": ");
                    format_expr(&property.value, out, 0);
                    out.push('\n');
                }
                indent(out, level + 1);
                out.push('}');
                if index + 1 < buttons.len() {
                    out.push_str("\n\n");
                } else {
                    out.push('\n');
                }
            }
            indent(out, level);
            out.push('}');
        }
        Item::Stmt(s) => format_stmt(s, out, level, comments),
    }
}

fn item_line(item: &Item) -> u32 {
    match item {
        Item::Function { loc, .. }
        | Item::Character { loc, .. }
        | Item::State { loc, .. }
        | Item::Scene { loc, .. }
        | Item::Screen { loc, .. } => loc.line,
        Item::Stmt(stmt) => stmt.loc().line,
    }
}

fn format_stmt(stmt: &Stmt, out: &mut String, level: usize, comments: &mut CommentCursor) {
    comments.emit_before(stmt.loc().line, out);
    match stmt {
        Stmt::Expr { expr, .. } => {
            indent(out, level);
            format_expr(expr, out, 0);
            out.push('\n');
        }
        Stmt::Let { name, ty, init, .. } => {
            indent(out, level);
            out.push_str("let ");
            out.push_str(name);
            if let Some(t) = ty {
                out.push_str(": ");
                out.push_str(t);
            }
            out.push_str(" = ");
            format_expr(init, out, 0);
            out.push('\n');
        }
        Stmt::Const { name, init, .. } => {
            indent(out, level);
            out.push_str("const ");
            out.push_str(name);
            out.push_str(" = ");
            format_expr(init, out, 0);
            out.push('\n');
        }
        Stmt::Block { body, .. } => {
            indent(out, level);
            out.push_str("{\n");
            for s in body {
                format_stmt(s, out, level + 1, comments);
            }
            indent(out, level);
            out.push_str("}\n");
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            indent(out, level);
            out.push_str("if ");
            format_expr(cond, out, 0);
            out.push(' ');
            format_stmt_blockish(then_body, out, level, comments);
            if let Some(e) = else_body {
                // else attaches to previous block end; ensure we're on a fresh line.
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                indent(out, level);
                out.push_str("else ");
                format_stmt_blockish(e, out, level, comments);
            }
        }
        Stmt::While { cond, body, .. } => {
            indent(out, level);
            out.push_str("while ");
            format_expr(cond, out, 0);
            out.push(' ');
            format_stmt_blockish(body, out, level, comments);
        }
        Stmt::Return { value, .. } => {
            indent(out, level);
            out.push_str("return");
            if let Some(v) = value {
                out.push(' ');
                format_expr(v, out, 0);
            }
            out.push('\n');
        }
        Stmt::Dialogue { speaker, text, .. } => {
            indent(out, level);
            if let Some(s) = speaker {
                out.push_str(s);
                out.push(' ');
            }
            out.push('"');
            out.push_str(&escape(text));
            out.push_str("\"\n");
        }
        Stmt::Jump { label, .. } => {
            indent(out, level);
            out.push_str("jump ");
            out.push_str(label);
            out.push('\n');
        }
        Stmt::Label { name, .. } => {
            indent(out, level);
            out.push_str("label ");
            out.push_str(name);
            out.push('\n');
        }
        Stmt::Choice { options, .. } => {
            indent(out, level);
            out.push_str("choice {\n");
            for arm in options {
                comments.emit_before(arm.loc.line, out);
                indent(out, level + 1);
                out.push('"');
                out.push_str(&escape(&arm.text));
                out.push_str("\" {\n");
                for s in &arm.body {
                    format_stmt(s, out, level + 2, comments);
                }
                indent(out, level + 1);
                out.push_str("}\n");
            }
            indent(out, level);
            out.push_str("}\n");
        }
        Stmt::Show { target, at, .. } => {
            indent(out, level);
            out.push_str("show ");
            out.push_str(target);
            if let Some(a) = at {
                out.push_str(" at ");
                out.push_str(a);
            }
            out.push('\n');
        }
        Stmt::Background { path, .. } => {
            indent(out, level);
            out.push_str("background \"");
            out.push_str(&escape(path));
            out.push_str("\"\n");
        }
        Stmt::Music { path, fade_in, .. } => {
            indent(out, level);
            out.push_str("music \"");
            out.push_str(&escape(path));
            out.push('"');
            if let Some(f) = fade_in {
                out.push_str(" fade_in ");
                out.push_str(&f.to_string());
            }
            out.push('\n');
        }
        Stmt::Hide { target, .. } => {
            indent(out, level);
            out.push_str("hide ");
            out.push_str(target);
            out.push('\n');
        }
        Stmt::End { ending, .. } => {
            indent(out, level);
            out.push_str("end");
            if let Some(e) = ending {
                out.push(' ');
                out.push('"');
                out.push_str(&escape(e));
                out.push('"');
            }
            out.push('\n');
        }
        Stmt::Call { target, .. } => {
            indent(out, level);
            out.push_str("call ");
            out.push_str(target);
            out.push('\n');
        }
        Stmt::HostCall { name, args, .. } => {
            indent(out, level);
            out.push_str("call ");
            out.push_str(name);
            for (key, value) in args {
                out.push(' ');
                out.push_str(key);
                out.push(' ');
                format_expr(value, out, 0);
            }
            out.push('\n');
        }
        Stmt::Transition { name, .. } => {
            indent(out, level);
            out.push_str("transition ");
            out.push_str(name);
            out.push('\n');
        }
        Stmt::Sound { path, .. } => {
            indent(out, level);
            out.push_str("sound \"");
            out.push_str(&escape(path));
            out.push_str("\"\n");
        }
        Stmt::Pause { seconds, .. } => {
            indent(out, level);
            out.push_str("pause");
            if let Some(seconds) = seconds {
                out.push(' ');
                out.push_str(&seconds.to_string());
            }
            out.push('\n');
        }
        Stmt::For {
            name, iter, body, ..
        } => {
            indent(out, level);
            out.push_str("for ");
            out.push_str(name);
            out.push_str(" in ");
            format_expr(iter, out, 0);
            out.push(' ');
            format_stmt_blockish(body, out, level, comments);
        }
        Stmt::Break { .. } => {
            indent(out, level);
            out.push_str("break\n");
        }
        Stmt::Continue { .. } => {
            indent(out, level);
            out.push_str("continue\n");
        }
    }
}

fn format_stmt_blockish(stmt: &Stmt, out: &mut String, level: usize, comments: &mut CommentCursor) {
    match stmt {
        Stmt::Block { body, .. } => {
            out.push_str("{\n");
            for s in body {
                format_stmt(s, out, level + 1, comments);
            }
            indent(out, level);
            out.push_str("}\n");
        }
        other => {
            out.push('\n');
            format_stmt(other, out, level + 1, comments);
        }
    }
}

/// Precedence for deciding when to parenthesize.
fn expr_prec(expr: &Expr) -> u8 {
    match expr {
        Expr::Binary { op, .. } => binop_prec(op),
        Expr::Unary { .. } => 12,
        Expr::Call { .. } | Expr::Field { .. } | Expr::Index { .. } => 14,
        _ => 15,
    }
}

fn binop_prec(op: &BinOp) -> u8 {
    match op {
        BinOp::Assign
        | BinOp::AddAssign
        | BinOp::SubAssign
        | BinOp::MulAssign
        | BinOp::DivAssign => 1,
        BinOp::Or => 2,
        BinOp::And => 3,
        BinOp::Eq | BinOp::Ne => 4,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 5,
        BinOp::Add | BinOp::Sub => 6,
        BinOp::Mul | BinOp::Div | BinOp::Rem => 7,
    }
}

fn format_expr(expr: &Expr, out: &mut String, parent_prec: u8) {
    let prec = expr_prec(expr);
    let needs_paren = prec < parent_prec;
    if needs_paren {
        out.push('(');
    }
    match expr {
        Expr::Null { .. } => out.push_str("null"),
        Expr::Bool { value, .. } => out.push_str(if *value { "true" } else { "false" }),
        Expr::Int { value, .. } => out.push_str(&value.to_string()),
        Expr::Float { value, .. } => out.push_str(&value.to_string()),
        Expr::String { value, .. } => {
            out.push('"');
            out.push_str(&escape(value));
            out.push('"');
        }
        Expr::Ident { name, .. } => out.push_str(name),
        Expr::List { elements, .. } => {
            out.push('[');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(e, out, 0);
            }
            out.push(']');
        }
        Expr::Map { entries, .. } => {
            out.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push('"');
                out.push_str(&escape(key));
                out.push_str("\": ");
                format_expr(value, out, 0);
            }
            out.push('}');
        }
        Expr::Unary { op, expr, .. } => {
            out.push(match op {
                UnaryOp::Neg => '-',
                UnaryOp::Not => '!',
            });
            format_expr(expr, out, 12);
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let p = binop_prec(op);
            format_expr(left, out, p);
            out.push(' ');
            out.push_str(binop(op));
            out.push(' ');
            // Right-assoc feel for assign; otherwise left-assoc with +1.
            let right_prec = if matches!(
                op,
                BinOp::Assign
                    | BinOp::AddAssign
                    | BinOp::SubAssign
                    | BinOp::MulAssign
                    | BinOp::DivAssign
            ) {
                p
            } else {
                p + 1
            };
            format_expr(right, out, right_prec);
        }
        Expr::Call { callee, args, .. } => {
            format_expr(callee, out, 14);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                format_expr(a, out, 0);
            }
            out.push(')');
        }
        Expr::Field { object, field, .. } => {
            format_expr(object, out, 14);
            out.push('.');
            out.push_str(field);
        }
        Expr::Index { object, index, .. } => {
            format_expr(object, out, 14);
            out.push('[');
            format_expr(index, out, 0);
            out.push(']');
        }
    }
    if needs_paren {
        out.push(')');
    }
}

fn binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Assign => "=",
        BinOp::AddAssign => "+=",
        BinOp::SubAssign => "-=",
        BinOp::MulAssign => "*=",
        BinOp::DivAssign => "/=",
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_function() {
        let src = "function add(a,b){return a+b}";
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("function add"));
        assert!(pretty.contains("return a + b"));
        assert!(pretty.contains("function add(a, b)"));
    }

    #[test]
    fn formats_scene() {
        let src = r#"scene main { "hi" jump other }"#;
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("scene main"));
        assert!(pretty.contains("jump other"));
    }

    #[test]
    fn blank_lines_between_top_level() {
        let src = "function a(){return 1}function b(){return 2}";
        let pretty = format_source(src).unwrap();
        assert!(
            pretty.contains("}\n\nfunction b"),
            "expected blank line between functions:\n{pretty}"
        );
    }

    #[test]
    fn indents_blocks() {
        let src = "function f(){if true{let x=1}}";
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("    if true"));
        assert!(pretty.contains("        let x = 1"));
    }

    #[test]
    fn spaces_around_operators() {
        let src = "function f(){return 1+2*3}";
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("return 1 + 2 * 3"), "pretty={pretty:?}");
    }

    #[test]
    fn formats_state_and_character() {
        let src = r#"state{trust:int=0}character hero{name:"A"}"#;
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("state {"));
        assert!(pretty.contains("trust: int = 0"));
        assert!(pretty.contains("character hero"));
        assert!(pretty.contains("name: \"A\""));
        assert!(pretty.contains("}\n\ncharacter"));
    }

    #[test]
    fn formats_lists_and_calls() {
        let src = "function f() {\nlet xs = [1,2]\nreturn abs(xs[0])\n}";
        let pretty = format_source(src).unwrap();
        assert!(pretty.contains("[1, 2]"), "pretty={pretty:?}");
        assert!(pretty.contains("abs("));
        assert!(pretty.contains("return abs(xs[0])"), "pretty={pretty:?}");
    }

    #[test]
    fn idempotent_on_pretty() {
        let src = r#"
function add(a, b) {
    return a + b
}

scene main {
    "hi"
}
"#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn preserves_vs3_edition_and_comments_idempotently() {
        let source = r#"// @edition 3
// module documentation
function add(a,b) {
    // keep this explanation
    return a+b // and this trailing note
}
"#;
        let once = format_source(source).unwrap();
        assert!(once.starts_with("// @edition 3"), "formatted={once:?}");
        assert!(once.contains("// module documentation"));
        assert!(once.contains("// keep this explanation"));
        assert!(once.contains("// and this trailing note"));
        assert_eq!(format_source(&once).unwrap(), once);
    }

    #[test]
    fn does_not_treat_url_inside_string_as_comment() {
        let source = r#"// @edition 3
function url() { return "https://velvet.dev/docs" }
"#;
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("\"https://velvet.dev/docs\""));
    }

    #[test]
    fn formats_host_and_presentation_commands_stably() {
        let source = r#"scene main {
            call combat.start enemy "goblin" count 2
            transition dissolve
            sound "audio/hit.ogg"
            pause 0.5
        }"#;
        let once = format_source(source).unwrap();
        assert!(once.contains("call combat.start enemy \"goblin\" count 2"));
        assert!(once.contains("transition dissolve"));
        assert!(once.contains("sound \"audio/hit.ogg\""));
        assert!(once.contains("pause 0.5"));
        assert_eq!(format_source(&once).unwrap(), once);
    }

    #[test]
    fn formats_declarative_screen_stably() {
        let source = r#"screen main_menu{title:"PLAY" button start{label:"START" action:"go" enabled:true}button quit{label:"QUIT" action:"quit"}}"#;
        let once = format_source(source).unwrap();
        assert!(once.contains("screen main_menu {"));
        assert!(once.contains("    title: \"PLAY\""));
        assert!(once.contains("    button start {"));
        assert!(once.contains("        enabled: true"));
        assert!(once.contains("    }\n\n    button quit {"));
        assert_eq!(format_source(&once).unwrap(), once);
    }
}

/// VS2 brace-aware format helpers.
pub mod vs2_format;
