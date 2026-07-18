//! Deterministic formatter for Velvet Story.

use crate::ast::*;
use crate::parser::parse;

/// Format options.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Spaces per indent level.
    pub indent: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { indent: 4 }
    }
}

/// Format source. On parse errors, returns best-effort normalized source.
pub fn format_source(source: &str) -> String {
    format_source_with(source, "format.vstory", &FormatOptions::default())
}

/// Format with file name for diagnostics (ignored for output).
pub fn format_source_with(source: &str, file: &str, opt: &FormatOptions) -> String {
    let parsed = parse(source, file);
    if parsed.file.items.is_empty() && !source.trim().is_empty() {
        // fallback: normalize trailing spaces only
        return normalize_whitespace(source);
    }
    let mut out = String::new();
    for (i, item) in parsed.file.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        format_top(item, &mut out, 0, opt);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn normalize_whitespace(source: &str) -> String {
    let mut lines: Vec<String> = source
        .lines()
        .map(|l| l.trim_end().to_string())
        .collect();
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

fn ind(opt: &FormatOptions, level: usize) -> String {
    " ".repeat(opt.indent * level)
}

fn format_top(item: &TopItem, out: &mut String, level: usize, opt: &FormatOptions) {
    match item {
        TopItem::Scene(sc) => {
            out.push_str(&format!("{}scene {}\n", ind(opt, level), sc.name));
            out.push('\n');
            for st in &sc.body {
                format_stmt(st, out, level, opt);
            }
        }
        TopItem::Include { path, .. } => {
            out.push_str(&format!("{}include \"{}\"\n", ind(opt, level), path));
        }
        TopItem::CharacterDecl { name, display, .. } => {
            out.push_str(&format!("{}character {}", ind(opt, level), name));
            if let Some(d) = display {
                out.push_str(&format!(" \"{d}\""));
            }
            out.push('\n');
        }
    }
}

fn format_stmt(st: &Stmt, out: &mut String, level: usize, opt: &FormatOptions) {
    let i = ind(opt, level);
    match st {
        Stmt::Background { id, .. } => out.push_str(&format!("{i}background {id}\n")),
        Stmt::Music { id, .. } => out.push_str(&format!("{i}music {id}\n")),
        Stmt::Sound { id, .. } => out.push_str(&format!("{i}sound {id}\n")),
        Stmt::Show {
            character,
            expression,
            at,
            ..
        } => {
            out.push_str(&format!("{i}show {character}"));
            if let Some(e) = expression {
                out.push(' ');
                out.push_str(e);
            }
            if let Some(a) = at {
                out.push_str(" at ");
                out.push_str(a);
            }
            out.push('\n');
        }
        Stmt::Hide { character, .. } => out.push_str(&format!("{i}hide {character}\n")),
        Stmt::Dialogue {
            speaker,
            msg_id,
            text,
            ..
        } => {
            out.push_str(&format!("{i}{speaker}"));
            if let Some(m) = msg_id {
                out.push_str(&format!(" @{m}"));
            }
            out.push_str(":\n");
            for line in text.lines() {
                out.push_str(&format!("{}{}\n", ind(opt, level + 1), line));
            }
            out.push('\n');
        }
        Stmt::Choice { options, .. } => {
            out.push_str(&format!("{i}choice:\n"));
            for o in options {
                out.push_str(&format!("{}\"{}\":\n", ind(opt, level + 1), escape_str(&o.label)));
                for s in &o.body {
                    format_stmt(s, out, level + 2, opt);
                }
                out.push('\n');
            }
        }
        Stmt::Goto { target, .. } => out.push_str(&format!("{i}goto {target}\n")),
        Stmt::CallScene { target, .. } => out.push_str(&format!("{i}call scene {target}\n")),
        Stmt::Return { .. } => out.push_str(&format!("{i}return\n")),
        Stmt::End { .. } => out.push_str(&format!("{i}end\n")),
        Stmt::Label { name, .. } => out.push_str(&format!("{i}label {name}\n")),
        Stmt::Set { name, value, .. } => {
            out.push_str(&format!("{i}set {name} = "));
            format_expr(value, out);
            out.push('\n');
        }
        Stmt::Add { name, value, .. } => {
            out.push_str(&format!("{i}add {name} "));
            format_expr(value, out);
            out.push('\n');
        }
        Stmt::Sub { name, value, .. } => {
            out.push_str(&format!("{i}sub {name} "));
            format_expr(value, out);
            out.push('\n');
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            out.push_str(&format!("{i}if "));
            format_expr(cond, out);
            out.push_str(":\n");
            for s in then_body {
                format_stmt(s, out, level + 1, opt);
            }
            if let Some(eb) = else_body {
                out.push_str(&format!("{i}else:\n"));
                for s in eb {
                    format_stmt(s, out, level + 1, opt);
                }
            }
            out.push('\n');
        }
        Stmt::CallCommand { name, args, .. } => {
            if args.is_empty() {
                out.push_str(&format!("{i}call {name}\n"));
            } else {
                out.push_str(&format!("{i}call {name}:\n"));
                for (k, v) in args {
                    out.push_str(&format!("{}{k}: ", ind(opt, level + 1)));
                    format_expr(v, out);
                    out.push('\n');
                }
            }
        }
        Stmt::Pause { duration, .. } => {
            out.push_str(&format!("{i}pause"));
            if let Some(d) = duration {
                out.push(' ');
                format_expr(d, out);
            }
            out.push('\n');
        }
        Stmt::Transition { name, .. } => out.push_str(&format!("{i}with {name}\n")),
        Stmt::Comment { text, .. } => {
            out.push_str(&format!("{i}{text}\n"));
        }
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_expr(e: &Expr, out: &mut String) {
    match e {
        Expr::Int(n, _) => out.push_str(&n.to_string()),
        Expr::Float(s, _) => out.push_str(s),
        Expr::Bool(b, _) => out.push_str(if *b { "true" } else { "false" }),
        Expr::Str(s, _) => out.push_str(&format!("\"{}\"", escape_str(s))),
        Expr::Ident(s, _) => out.push_str(s),
        Expr::Binary {
            op, left, right, ..
        } => {
            format_expr(left, out);
            out.push(' ');
            out.push_str(match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "and",
                BinOp::Or => "or",
            });
            out.push(' ');
            format_expr(right, out);
        }
        Expr::Unary { op, expr, .. } => {
            out.push_str(match op {
                UnaryOp::Not => "not ",
                UnaryOp::Neg => "-",
            });
            format_expr(expr, out);
        }
    }
}

/// Idempotence check helper.
pub fn is_idempotent(source: &str) -> bool {
    let a = format_source(source);
    let b = format_source(&a);
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_idempotent_welcome() {
        let src = r#"
scene start
background bedroom
luna:
    Hola
set affection = 0
"#;
        assert!(is_idempotent(src));
    }
}
