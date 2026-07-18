//! # velvet-script-format
//!
//! Deterministic formatter for Velvet Script (pretty-print from AST).

#![deny(missing_docs)]

use velvet_script_ast::{BinOp, Expr, Item, Module, Stmt, UnaryOp};
use velvet_script_parser::{parse_file, ParseError};

/// Format source; returns pretty string or parse error.
pub fn format_source(source: &str) -> Result<String, ParseError> {
    let parsed = parse_file(source, None)?;
    Ok(format_module(&parsed.module))
}

/// Format a module AST.
///
/// Top-level items are separated by a blank line. Blocks are indented with
/// four spaces. Binary operators are padded with spaces.
pub fn format_module(module: &Module) -> String {
    let mut out = String::new();
    for (i, item) in module.items.iter().enumerate() {
        if i > 0 {
            // Blank line between top-level items.
            out.push('\n');
        }
        format_item(item, &mut out, 0);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

fn format_item(item: &Item, out: &mut String, level: usize) {
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
                format_stmt(s, out, level + 1);
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
                format_stmt(s, out, level + 1);
            }
            indent(out, level);
            out.push('}');
        }
        Item::Stmt(s) => format_stmt(s, out, level),
    }
}

fn format_stmt(stmt: &Stmt, out: &mut String, level: usize) {
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
                format_stmt(s, out, level + 1);
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
            format_stmt_blockish(then_body, out, level);
            if let Some(e) = else_body {
                // else attaches to previous block end; ensure we're on a fresh line.
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                indent(out, level);
                out.push_str("else ");
                format_stmt_blockish(e, out, level);
            }
        }
        Stmt::While { cond, body, .. } => {
            indent(out, level);
            out.push_str("while ");
            format_expr(cond, out, 0);
            out.push(' ');
            format_stmt_blockish(body, out, level);
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
                indent(out, level + 1);
                out.push('"');
                out.push_str(&escape(&arm.text));
                out.push_str("\" {\n");
                for s in &arm.body {
                    format_stmt(s, out, level + 2);
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
        Stmt::For {
            name, iter, body, ..
        } => {
            indent(out, level);
            out.push_str("for ");
            out.push_str(name);
            out.push_str(" in ");
            format_expr(iter, out, 0);
            out.push(' ');
            format_stmt_blockish(body, out, level);
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

fn format_stmt_blockish(stmt: &Stmt, out: &mut String, level: usize) {
    match stmt {
        Stmt::Block { body, .. } => {
            out.push_str("{\n");
            for s in body {
                format_stmt(s, out, level + 1);
            }
            indent(out, level);
            out.push_str("}\n");
        }
        other => {
            out.push('\n');
            format_stmt(other, out, level + 1);
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
        assert!(pretty.contains("1 + 2 * 3") || pretty.contains("1 + (2 * 3)"));
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
        assert!(pretty.contains("xs[0]") || pretty.contains("xs[0]"));
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
}

/// VS2 brace-aware format helpers.
pub mod vs2_format;
