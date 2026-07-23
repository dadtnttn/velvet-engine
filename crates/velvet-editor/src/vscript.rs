//! Velvet Script (VScript) — Studio advanced language for buttons, layers, flow.
//!
//! Designed to sit inside `@advanced` blocks and freeform `.vel` scripts.
//! Compatible with existing lines (`game.new()`, `scene.open(...)`, `jump`, …)
//! and adds first-class **layer** / **button** / **graph** calls.

use std::fmt;

/// A single VScript statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `game.new()`
    GameNew,
    /// `game.load_last()`
    GameLoadLast,
    /// `game.quit()`
    GameQuit,
    /// `game.save()` / `game.load("slot")`
    GameSave,
    GameLoad {
        /// Optional slot name.
        slot: Option<String>,
    },
    /// `scene.open("path")` / `scene.open(path)`
    SceneOpen {
        /// Target scene path or id.
        target: String,
    },
    /// `jump target`
    Jump {
        /// Label / scene.
        target: String,
    },
    /// `call target`
    Call {
        /// Subroutine target.
        target: String,
    },
    /// `layer.open("id")` — switch Studio / runtime layer.
    LayerOpen {
        /// Layer id.
        id: String,
    },
    /// `layer.show("id")` — make layer visible without exclusive focus.
    LayerShow {
        /// Layer id.
        id: String,
    },
    /// `layer.hide("id")`
    LayerHide {
        /// Layer id.
        id: String,
    },
    /// `layer.lock("id")` / `layer.unlock("id")`
    LayerLock {
        /// Layer id.
        id: String,
        /// Lock when true.
        locked: bool,
    },
    /// `button.press("button.start")` — invoke button handler.
    ButtonPress {
        /// Region id.
        id: String,
    },
    /// `button.focus("button.start")`
    ButtonFocus {
        /// Region id.
        id: String,
    },
    /// `button.set_text("button.start", "Play")`
    ButtonSetText {
        /// Region id.
        id: String,
        /// New label.
        text: String,
    },
    /// `connect layer_a -> layer_b` (graph edge helper).
    Connect {
        /// Source layer.
        from: String,
        /// Target layer.
        to: String,
    },
    /// `if cond {`  (condition open — body not nested in this lightweight model)
    If {
        /// Condition expression text.
        cond: String,
    },
    /// Assignment `name = value`
    SetVar {
        /// Variable name.
        name: String,
        /// Value expression.
        value: String,
    },
    /// `// comment`
    Comment {
        /// Comment body.
        text: String,
    },
    /// Unknown / passthrough raw line (preserves advanced body).
    Raw {
        /// Original line.
        line: String,
    },
}

impl Stmt {
    /// Emit source form.
    pub fn emit(&self) -> String {
        match self {
            Self::GameNew => "game.new()".into(),
            Self::GameLoadLast => "game.load_last()".into(),
            Self::GameQuit => "game.quit()".into(),
            Self::GameSave => "game.save()".into(),
            Self::GameLoad { slot: None } => "game.load()".into(),
            Self::GameLoad { slot: Some(s) } => format!("game.load(\"{s}\")"),
            Self::SceneOpen { target } => format!("scene.open(\"{target}\")"),
            Self::Jump { target } => format!("jump {target}"),
            Self::Call { target } => format!("call {target}"),
            Self::LayerOpen { id } => format!("layer.open(\"{id}\")"),
            Self::LayerShow { id } => format!("layer.show(\"{id}\")"),
            Self::LayerHide { id } => format!("layer.hide(\"{id}\")"),
            Self::LayerLock { id, locked: true } => format!("layer.lock(\"{id}\")"),
            Self::LayerLock { id, locked: false } => format!("layer.unlock(\"{id}\")"),
            Self::ButtonPress { id } => format!("button.press(\"{id}\")"),
            Self::ButtonFocus { id } => format!("button.focus(\"{id}\")"),
            Self::ButtonSetText { id, text } => format!("button.set_text(\"{id}\", \"{text}\")"),
            Self::Connect { from, to } => format!("connect {from} -> {to}"),
            Self::If { cond } => format!("if {cond} {{"),
            Self::SetVar { name, value } => format!("{name} = {value}"),
            Self::Comment { text } => format!("// {text}"),
            Self::Raw { line } => line.clone(),
        }
    }
}

/// Parse one line into a statement (best-effort).
pub fn parse_line(line: &str) -> Stmt {
    let t = line.trim();
    if t.is_empty() {
        return Stmt::Raw {
            line: String::new(),
        };
    }
    if let Some(rest) = t.strip_prefix("//") {
        return Stmt::Comment {
            text: rest.trim().into(),
        };
    }
    if t == "game.new()" || t == "game.new( )" {
        return Stmt::GameNew;
    }
    if t == "game.load_last()" {
        return Stmt::GameLoadLast;
    }
    if t == "game.quit()" {
        return Stmt::GameQuit;
    }
    if t == "game.save()" {
        return Stmt::GameSave;
    }
    if let Some(inner) = strip_call(t, "game.load") {
        let slot = parse_string_arg(&inner);
        return Stmt::GameLoad { slot };
    }
    if let Some(inner) = strip_call(t, "scene.open") {
        if let Some(target) = parse_string_arg(&inner) {
            return Stmt::SceneOpen { target };
        }
    }
    if let Some(rest) = t.strip_prefix("jump ") {
        return Stmt::Jump {
            target: rest.trim().into(),
        };
    }
    if let Some(rest) = t.strip_prefix("call ") {
        return Stmt::Call {
            target: rest.trim().into(),
        };
    }
    if let Some(inner) = strip_call(t, "layer.open") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::LayerOpen { id };
        }
    }
    if let Some(inner) = strip_call(t, "layer.show") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::LayerShow { id };
        }
    }
    if let Some(inner) = strip_call(t, "layer.hide") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::LayerHide { id };
        }
    }
    if let Some(inner) = strip_call(t, "layer.lock") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::LayerLock { id, locked: true };
        }
    }
    if let Some(inner) = strip_call(t, "layer.unlock") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::LayerLock { id, locked: false };
        }
    }
    if let Some(inner) = strip_call(t, "button.press") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::ButtonPress { id };
        }
    }
    if let Some(inner) = strip_call(t, "button.focus") {
        if let Some(id) = parse_string_arg(&inner) {
            return Stmt::ButtonFocus { id };
        }
    }
    if let Some(inner) = strip_call(t, "button.set_text") {
        let parts = split_args(&inner);
        if parts.len() >= 2 {
            return Stmt::ButtonSetText {
                id: unquote(&parts[0]),
                text: unquote(&parts[1]),
            };
        }
    }
    if let Some(rest) = t.strip_prefix("connect ") {
        if let Some((a, b)) = rest.split_once("->") {
            return Stmt::Connect {
                from: a.trim().into(),
                to: b.trim().into(),
            };
        }
    }
    if let Some(rest) = t.strip_prefix("if ") {
        let cond = rest.trim().trim_end_matches('{').trim().to_string();
        return Stmt::If { cond };
    }
    if let Some((name, value)) = t.split_once('=') {
        let name = name.trim();
        if !name.is_empty() && !name.contains('(') && !name.contains(' ') {
            return Stmt::SetVar {
                name: name.into(),
                value: value.trim().into(),
            };
        }
    }
    Stmt::Raw { line: t.into() }
}

/// Parse a multi-line script body.
pub fn parse_script(source: &str) -> Vec<Stmt> {
    source.lines().map(parse_line).collect()
}

/// Emit script from statements.
pub fn emit_script(stmts: &[Stmt]) -> String {
    let mut out = String::new();
    for s in stmts {
        let line = s.emit();
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        } else {
            out.push('\n');
        }
    }
    out
}

/// Validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptIssue {
    /// 1-based line.
    pub line: usize,
    /// Message.
    pub message: String,
}

impl fmt::Display for ScriptIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}: {}", self.line, self.message)
    }
}

/// Validate statements against known layers and button ids.
pub fn validate(stmts: &[Stmt], known_layers: &[&str], known_buttons: &[&str]) -> Vec<ScriptIssue> {
    let mut issues = Vec::new();
    for (i, s) in stmts.iter().enumerate() {
        let line = i + 1;
        match s {
            Stmt::LayerOpen { id }
            | Stmt::LayerShow { id }
            | Stmt::LayerHide { id }
            | Stmt::LayerLock { id, .. } => {
                if !known_layers.is_empty() && !known_layers.iter().any(|k| *k == id) {
                    issues.push(ScriptIssue {
                        line,
                        message: format!("unknown layer \"{id}\""),
                    });
                }
            }
            Stmt::ButtonPress { id }
            | Stmt::ButtonFocus { id }
            | Stmt::ButtonSetText { id, .. } => {
                if !known_buttons.is_empty() && !known_buttons.iter().any(|k| *k == id) {
                    issues.push(ScriptIssue {
                        line,
                        message: format!("unknown button \"{id}\""),
                    });
                }
            }
            Stmt::Connect { from, to } => {
                if !known_layers.is_empty() {
                    if !known_layers.iter().any(|k| *k == from) {
                        issues.push(ScriptIssue {
                            line,
                            message: format!("connect: unknown from \"{from}\""),
                        });
                    }
                    if !known_layers.iter().any(|k| *k == to) {
                        issues.push(ScriptIssue {
                            line,
                            message: format!("connect: unknown to \"{to}\""),
                        });
                    }
                }
            }
            Stmt::Raw { line: raw } if !raw.is_empty() && !raw.starts_with('}') => {
                // soft warn only for obvious typos
                if raw.contains("layer.") && !raw.contains('(') {
                    issues.push(ScriptIssue {
                        line,
                        message: format!("looks incomplete: {raw}"),
                    });
                }
            }
            _ => {}
        }
    }
    issues
}

/// Builtin API catalog for the Script panel (insert snippets).
pub fn api_catalog() -> &'static [(&'static str, &'static str, &'static str)] {
    // (category, snippet, description)
    &[
        ("game", "game.new()", "Start a new game"),
        ("game", "game.load_last()", "Load last save"),
        ("game", "game.quit()", "Quit application"),
        ("game", "game.save()", "Save game"),
        ("game", "game.load(\"slot1\")", "Load named slot"),
        (
            "scene",
            "scene.open(\"scripts/main.vel\")",
            "Open scene file",
        ),
        ("flow", "jump chapter_2", "Jump to label/scene"),
        ("flow", "call subroutine", "Call subroutine"),
        ("layer", "layer.open(\"menu_settings\")", "Switch to layer"),
        ("layer", "layer.show(\"hud\")", "Show layer"),
        ("layer", "layer.hide(\"hud\")", "Hide layer"),
        ("layer", "layer.lock(\"main_menu\")", "Lock layer"),
        ("layer", "layer.unlock(\"main_menu\")", "Unlock layer"),
        (
            "button",
            "button.press(\"button.start\")",
            "Fire button handler",
        ),
        ("button", "button.focus(\"button.start\")", "Focus button"),
        (
            "button",
            "button.set_text(\"button.start\", \"Play\")",
            "Change button label",
        ),
        (
            "graph",
            "connect main_menu -> scene",
            "Link layers in graph",
        ),
        ("logic", "if flags.met_aria {", "Conditional branch"),
        ("vars", "flags.met_aria = true", "Set variable"),
    ]
}

/// Build on_pressed body that opens a layer (for graph connections).
pub fn snippet_open_layer(layer_id: &str) -> String {
    format!("layer.open(\"{layer_id}\")\n")
}

/// Build connect + open pair.
pub fn snippet_connect_and_open(from: &str, to: &str) -> String {
    format!("connect {from} -> {to}\nlayer.open(\"{to}\")\n")
}

fn strip_call(line: &str, name: &str) -> Option<String> {
    let line = line.trim();
    let prefix = format!("{name}(");
    if let Some(rest) = line.strip_prefix(&prefix) {
        if let Some(inner) = rest.strip_suffix(')') {
            return Some(inner.trim().to_string());
        }
    }
    None
}

fn parse_string_arg(inner: &str) -> Option<String> {
    let t = inner.trim();
    if t.is_empty() {
        return None;
    }
    // first string or bare ident
    if let Some(stripped) = t.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_string());
    }
    let ident = t.split(',').next()?.trim();
    if !ident.is_empty() {
        return Some(ident.trim_matches('"').to_string());
    }
    None
}

fn split_args(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    for ch in inner.chars() {
        match ch {
            '"' => {
                in_str = !in_str;
                cur.push(ch);
            }
            ',' if !in_str => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Keyword tint category for syntax highlight in Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxKind {
    /// Default.
    Normal,
    /// Comment.
    Comment,
    /// game/scene/layer/button keywords.
    Keyword,
    /// String literal.
    String,
    /// jump/call/connect/if.
    Flow,
}

/// Classify a whole line for simple highlighting.
pub fn classify_line(line: &str) -> SyntaxKind {
    let t = line.trim();
    if t.starts_with("//") || t.starts_with("// @") {
        return SyntaxKind::Comment;
    }
    if t.starts_with("jump ")
        || t.starts_with("call ")
        || t.starts_with("connect ")
        || t.starts_with("if ")
    {
        return SyntaxKind::Flow;
    }
    if t.starts_with("game.")
        || t.starts_with("scene.")
        || t.starts_with("layer.")
        || t.starts_with("button.")
    {
        return SyntaxKind::Keyword;
    }
    if t.contains('"') {
        return SyntaxKind::String;
    }
    SyntaxKind::Normal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_emit_layer_button() {
        let s = parse_line("layer.open(\"menu_settings\")");
        assert_eq!(
            s,
            Stmt::LayerOpen {
                id: "menu_settings".into()
            }
        );
        assert_eq!(s.emit(), "layer.open(\"menu_settings\")");

        let b = parse_line("button.press(\"button.start\")");
        assert_eq!(
            b,
            Stmt::ButtonPress {
                id: "button.start".into()
            }
        );
    }

    #[test]
    fn parse_legacy_game() {
        assert_eq!(parse_line("game.new()"), Stmt::GameNew);
        assert!(matches!(
            parse_line("scene.open(\"scripts/main.vel\")"),
            Stmt::SceneOpen { .. }
        ));
    }

    #[test]
    fn validate_unknown_layer() {
        let stmts = parse_script("layer.open(\"nope\")\nbutton.press(\"button.start\")\n");
        let issues = validate(&stmts, &["main_menu"], &["button.start"]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("layer"));
    }

    #[test]
    fn connect_roundtrip() {
        let s = parse_line("connect main_menu -> scene");
        assert_eq!(
            s,
            Stmt::Connect {
                from: "main_menu".into(),
                to: "scene".into()
            }
        );
        assert_eq!(s.emit(), "connect main_menu -> scene");
    }

    #[test]
    fn api_catalog_snippets_are_unique_parseable_and_documented() {
        let catalog = api_catalog();
        let snippets: std::collections::HashSet<_> =
            catalog.iter().map(|(_, snippet, _)| *snippet).collect();
        assert_eq!(snippets.len(), catalog.len(), "duplicate insertion snippet");
        for (category, snippet, description) in catalog {
            assert!(!category.is_empty());
            assert!(!description.is_empty());
            assert!(
                !matches!(parse_line(snippet), Stmt::Raw { .. }),
                "catalog snippet is not parsed: {snippet}"
            );
        }
        for required in [
            "game", "scene", "flow", "layer", "button", "graph", "logic", "vars",
        ] {
            assert!(
                catalog.iter().any(|(category, _, _)| *category == required),
                "missing category {required}"
            );
        }
    }
}
