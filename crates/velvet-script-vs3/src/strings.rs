//! Translatable-string extraction for VS3 sources.
//!
//! Games keep their text in the source language and translate it when it is
//! shown (`tr`). This pass lists every string a script can show: plain
//! literals become exact entries, and concatenations become templates with
//! numbered holes (`"Tienes " + str(n) + " años"` → `Tienes {0} años`), each
//! hole described by the expression that fills it. `{marker}` placeholders that
//! scripts substitute themselves (`"Invitarl{xlo} a salir"`) become holes too.

use std::collections::HashMap;

use velvet_script_ast::{BinOp, Expr, Item, Stmt};
use velvet_script_parser::parse_file;

/// One translatable string (deduplicated by `key`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    /// Source text; templates use `{0}`, `{1}`... for runtime values.
    pub key: String,
    /// What fills each hole (source expression or `{marker}`), by hole number.
    pub holes: Vec<String>,
    /// Where it appears (`file:line`), up to three places.
    pub places: Vec<String>,
    /// Enclosing call / map key, e.g. `open_choice#2` or `title`.
    pub context: String,
    /// Times it appears.
    pub count: usize,
}

/// Map keys whose string values are identifiers, never shown text.
const ID_KEYS: &[&str] = &[
    "id", "op", "act", "action", "kind", "k", "font", "color", "fill", "stroke", "icon", "tag",
    "type", "key", "mode", "align", "anchor", "handler", "h", "route", "src", "img", "svg",
    "sound", "flag", "cat", "slot", "shape", "style", "bg", "fg", "col", "c", "ico", "vibe",
    "code", "iso", "region", "tz", "screen", "tab", "app", "purpose", "path", "file", "event",
];

/// Calls whose string arguments are identifiers.
const ID_CALLS: &[&str] = &[
    "emit", "map_get", "map_has", "map_set", "map_remove", "has_flag", "set_flag", "flag",
    "clear_flag", "list_contains", "str_replace", "str_split", "str_starts_with",
    "str_ends_with", "str_contains", "str_find", "str_slice", "type_of", "svg", "img",
    "go", "goto", "screen", "set_screen", "sfx", "play_sound", "music", "rgba", "hex",
];

#[derive(Default)]
struct Collector {
    file: String,
    entries: HashMap<String, StringEntry>,
    order: Vec<String>,
}

fn is_identifier_like(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    if s.contains('<') && s.contains('>') {
        return true;
    }
    let ascii_id = s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_:.-/".contains(c));
    ascii_id && s.chars().any(|c| "_:./".contains(c))
}

fn has_letters(s: &str) -> bool {
    s.chars().any(char::is_alphabetic)
}

/// Compact source-like rendering of a hole expression.
fn describe(e: &Expr) -> String {
    let s = match e {
        Expr::Null { .. } => "null".into(),
        Expr::Bool { value, .. } => value.to_string(),
        Expr::Int { value, .. } => value.to_string(),
        Expr::Float { value, .. } => value.to_string(),
        Expr::String { value, .. } => format!("{value:?}"),
        Expr::Ident { name, .. } => name.clone(),
        Expr::List { .. } => "[...]".into(),
        Expr::Map { .. } => "{...}".into(),
        Expr::Unary { expr, .. } => format!("-{}", describe(expr)),
        Expr::Binary { left, right, .. } => format!("{} op {}", describe(left), describe(right)),
        Expr::Call { callee, args, .. } => format!(
            "{}({})",
            describe(callee),
            args.iter().map(describe).collect::<Vec<_>>().join(", ")
        ),
        Expr::Field { object, field, .. } => format!("{}.{field}", describe(object)),
        Expr::Index { object, index, .. } => format!("{}[{}]", describe(object), describe(index)),
    };
    if s.chars().count() > 48 {
        let cut: String = s.chars().take(45).collect();
        format!("{cut}...")
    } else {
        s
    }
}

fn callee_name(e: &Expr) -> String {
    match e {
        Expr::Ident { name, .. } => name.clone(),
        Expr::Field { field, .. } => field.clone(),
        _ => String::new(),
    }
}

enum Piece<'a> {
    Lit(String),
    Hole(&'a Expr),
}

fn flatten_add<'a>(e: &'a Expr, out: &mut Vec<Piece<'a>>) {
    match e {
        Expr::Binary {
            left,
            op: BinOp::Add,
            right,
            ..
        } => {
            flatten_add(left, out);
            flatten_add(right, out);
        }
        Expr::String { value, .. } => out.push(Piece::Lit(value.clone())),
        other => out.push(Piece::Hole(other)),
    }
}

fn is_string_concat(e: &Expr) -> bool {
    match e {
        Expr::Binary {
            left,
            op: BinOp::Add,
            right,
            ..
        } => {
            matches!(**left, Expr::String { .. })
                || matches!(**right, Expr::String { .. })
                || is_string_concat(left)
                || is_string_concat(right)
        }
        _ => false,
    }
}

/// Split `{marker}` placeholders out of a literal. `l{xlo}` / `l{plo}` are
/// replaced as a whole by scripts ("Invitarl{xlo}" → "Invitarla").
fn split_markers(lit: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut rest = lit;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = &after[..close];
        let valid = !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && !name.chars().all(|c| c.is_ascii_digit());
        if !valid {
            out.push((false, rest[..open + 1 + close + 1].to_string()));
            rest = &after[close + 1..];
            continue;
        }
        let mut before = rest[..open].to_string();
        if (name == "xlo" || name == "plo") && before.ends_with('l') {
            before.pop();
        }
        if !before.is_empty() {
            out.push((false, before));
        }
        out.push((true, format!("{{{name}}}")));
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        out.push((false, rest.to_string()));
    }
    out
}

impl Collector {
    fn add(&mut self, key: String, holes: Vec<String>, line: u32, context: &str) {
        let place = format!("{}:{line}", self.file);
        match self.entries.get_mut(&key) {
            Some(entry) => {
                entry.count += 1;
                if entry.places.len() < 3 {
                    entry.places.push(place);
                }
                if entry.context.is_empty() {
                    entry.context = context.to_string();
                }
            }
            None => {
                self.order.push(key.clone());
                self.entries.insert(
                    key.clone(),
                    StringEntry {
                        key,
                        holes,
                        places: vec![place],
                        context: context.to_string(),
                        count: 1,
                    },
                );
            }
        }
    }

    fn literal(&mut self, value: &str, line: u32, context: &str) {
        if !has_letters(value) || is_identifier_like(value) {
            return;
        }
        let parts = split_markers(value);
        if parts.iter().any(|(m, _)| *m) {
            self.template(parts.into_iter().map(|(m, s)| (m, s, None)).collect(), line, context);
        } else {
            self.add(value.to_string(), Vec::new(), line, context);
        }
    }

    /// Pieces: (is_hole, literal text or marker, hole expression).
    fn template(&mut self, pieces: Vec<(bool, String, Option<String>)>, line: u32, context: &str) {
        let mut key = String::new();
        let mut holes: Vec<String> = Vec::new();
        let mut last_hole = false;
        let mut letters = 0usize;
        for (is_hole, text, expr) in pieces {
            if is_hole {
                if last_hole {
                    // Adjacent holes would be ambiguous: merge them.
                    if let Some(h) = holes.last_mut() {
                        h.push_str(" + ");
                        h.push_str(&expr.unwrap_or(text));
                    }
                    continue;
                }
                key.push_str(&format!("{{{}}}", holes.len()));
                holes.push(expr.unwrap_or(text));
                last_hole = true;
            } else {
                letters += text.chars().filter(|c| c.is_alphabetic()).count();
                key.push_str(&text);
                last_hole = false;
            }
        }
        if holes.is_empty() {
            if letters > 0 {
                self.add(key, holes, line, context);
            }
            return;
        }
        let structural = letters == 0;
        if structural {
            // "{0}: {1}" style joins are kept (their parts translate on their
            // own) when they carry punctuation; bare spaces would match anything.
            let lits: String = key
                .split(|c| c == '{' || c == '}')
                .filter(|s| !s.chars().all(|c| c.is_ascii_digit()))
                .collect();
            if !lits.chars().any(|c| !c.is_whitespace()) {
                return;
            }
        }
        self.add(key, holes, line, context);
    }

    fn concat(&mut self, e: &Expr, context: &str) {
        let mut pieces = Vec::new();
        flatten_add(e, &mut pieces);
        let line = e.loc().line;
        let mut out = Vec::new();
        for piece in &pieces {
            match piece {
                Piece::Lit(s) => {
                    for (m, t) in split_markers(s) {
                        out.push((m, t, None));
                    }
                }
                Piece::Hole(h) => out.push((true, String::new(), Some(describe(h)))),
            }
        }
        self.template(out, line, context);
        // Holes may hold strings of their own (calls, conditionals...).
        for piece in pieces {
            if let Piece::Hole(h) = piece {
                self.expr(h, context);
            }
        }
    }

    fn expr(&mut self, e: &Expr, context: &str) {
        match e {
            Expr::String { value, loc } => self.literal(value, loc.line, context),
            Expr::List { elements, .. } => {
                for el in elements {
                    self.expr(el, context);
                }
            }
            Expr::Map { entries, .. } => {
                for (k, v) in entries {
                    if ID_KEYS.contains(&k.as_str()) {
                        if let Expr::String { .. } = v {
                            continue;
                        }
                    }
                    self.expr(v, k);
                }
            }
            Expr::Unary { expr, .. } => self.expr(expr, context),
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinOp::Add if is_string_concat(e) => self.concat(e, context),
                BinOp::Eq | BinOp::Ne => {
                    // Comparisons test identifiers, not shown text.
                    for side in [left, right] {
                        if !matches!(**side, Expr::String { .. }) {
                            self.expr(side, context);
                        }
                    }
                }
                BinOp::AddAssign if matches!(**right, Expr::String { .. }) || is_string_concat(right) => {
                    let mut pieces = vec![(true, String::new(), Some(describe(left)))];
                    let mut flat = Vec::new();
                    flatten_add(right, &mut flat);
                    for p in &flat {
                        match p {
                            Piece::Lit(s) => {
                                for (m, t) in split_markers(s) {
                                    pieces.push((m, t, None));
                                }
                            }
                            Piece::Hole(h) => pieces.push((true, String::new(), Some(describe(h)))),
                        }
                    }
                    self.template(pieces, e.loc().line, context);
                    self.expr(right, context);
                }
                _ => {
                    self.expr(left, context);
                    self.expr(right, context);
                }
            },
            Expr::Call { callee, args, .. } => {
                let name = callee_name(callee);
                let skip_strings = ID_CALLS.contains(&name.as_str());
                for (i, a) in args.iter().enumerate() {
                    if skip_strings && matches!(a, Expr::String { .. }) {
                        continue;
                    }
                    let ctx = format!("{name}#{i}");
                    self.expr(a, &ctx);
                }
            }
            Expr::Field { object, .. } => self.expr(object, context),
            Expr::Index { object, index, .. } => {
                self.expr(object, context);
                if !matches!(**index, Expr::String { .. }) {
                    self.expr(index, context);
                }
            }
            Expr::Null { .. }
            | Expr::Bool { .. }
            | Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Ident { .. } => {}
        }
    }

    fn stmt(&mut self, s: &Stmt, func: &str) {
        match s {
            Stmt::Expr { expr, .. } => self.expr(expr, func),
            Stmt::Let { name, init, .. } | Stmt::Const { name, init, .. } => self.expr(init, name),
            Stmt::Block { body, .. } => self.body(body, func),
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                self.expr(cond, func);
                self.stmt(then_body, func);
                if let Some(e) = else_body {
                    self.stmt(e, func);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.expr(cond, func);
                self.stmt(body, func);
            }
            Stmt::For { iter, body, .. } => {
                self.expr(iter, func);
                self.stmt(body, func);
            }
            Stmt::Return { value: Some(v), .. } => self.expr(v, func),
            Stmt::Dialogue { text, loc, .. } => self.literal(text, loc.line, func),
            _ => {}
        }
    }

    fn body(&mut self, body: &[Stmt], func: &str) {
        for s in body {
            self.stmt(s, func);
        }
    }
}

/// Extract the translatable strings of one source file, in first-seen order.
pub fn extract_strings(source: &str, file: &str) -> Result<Vec<StringEntry>, String> {
    let parsed = parse_file(source, Some(file)).map_err(|e| e.to_string())?;
    let mut c = Collector {
        file: file.to_string(),
        ..Default::default()
    };
    for item in &parsed.module.items {
        match item {
            Item::Function { name, body, .. } => c.body(body, name),
            Item::Stmt(s) => c.stmt(s, ""),
            _ => {}
        }
    }
    let mut entries = c.entries;
    Ok(c.order
        .into_iter()
        .filter_map(|k| entries.remove(&k))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_templates_and_markers() {
        let src = r#"// @edition 3
function f(p: map, n: int) {
    if p.kind == "amigo" { return null }
    text("Hola mundo", 10, 20)
    let s = "Tienes " + str(n) + " años"
    let t = fmt("Invitarl{xlo} a salir con {x}", {})
    emit("audio.play", {"sound": "clic"})
    let u = p.first + " " + p.last
    let v = p.first + ": " + p.last
    return push({"op": "text", "text": "Listo"})
}
"#;
        let out = extract_strings(src, "a.vel").unwrap();
        let keys: Vec<&str> = out.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "Hola mundo",
                "Tienes {0} años",
                "Invitar{0} a salir con {1}",
                "{0}: {1}",
                "Listo"
            ]
        );
        assert_eq!(out[1].holes, vec!["str(n)"]);
        assert_eq!(out[2].holes, vec!["{xlo}", "{x}"]);
    }
}
