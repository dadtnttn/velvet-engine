//! VS2 LSP helpers: completions, hover, semantic tokens for rust-like surface.

#![allow(missing_docs)]
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vs2CompletionKind {
    Keyword,
    Function,
    Type,
    Variable,
    Module,
    Snippet,
    Layer,
    Scene,
    MsgKey,
}

#[derive(Debug, Clone)]
pub struct Vs2Completion {
    pub label: String,
    pub kind: Vs2CompletionKind,
    pub detail: String,
    pub insert: String,
}

impl Vs2Completion {
    pub fn kw(label: &str) -> Self {
        Self {
            label: label.into(),
            kind: Vs2CompletionKind::Keyword,
            detail: "keyword".into(),
            insert: label.into(),
        }
    }
    pub fn fn_item(label: &str, detail: &str) -> Self {
        Self {
            label: label.into(),
            kind: Vs2CompletionKind::Function,
            detail: detail.into(),
            insert: format!("{label}($0)"),
        }
    }
    pub fn ty(label: &str) -> Self {
        Self {
            label: label.into(),
            kind: Vs2CompletionKind::Type,
            detail: "type".into(),
            insert: label.into(),
        }
    }
}

pub static VS2_KEYWORDS: &[&str] = &[
    "fn",
    "struct",
    "enum",
    "mod",
    "use",
    "pub",
    "let",
    "mut",
    "const",
    "static",
    "if",
    "else",
    "while",
    "for",
    "loop",
    "match",
    "return",
    "break",
    "continue",
    "impl",
    "trait",
    "type",
    "where",
    "as",
    "in",
    "ref",
    "move",
    "scene",
    "say",
    "menu",
    "jump",
    "call",
    "show",
    "hide",
    "with",
    "at",
    "character",
    "screen",
    "state",
    "transform",
    "layer",
    "true",
    "false",
    "self",
    "Self",
    "crate",
    "super",
];

pub static VS2_TYPES: &[&str] = &[
    "i32",
    "i64",
    "u32",
    "u64",
    "f32",
    "f64",
    "bool",
    "str",
    "String",
    "Option",
    "Result",
    "Vec",
    "LayerId",
    "SceneId",
    "MsgId",
    "ScriptError",
    "Transform",
    "Transition",
    "Color",
    "Vec2",
];

pub fn story_snippets() -> Vec<Vs2Completion> {
    vec![
        Vs2Completion {
            label: "scene".into(),
            kind: Vs2CompletionKind::Snippet,
            detail: "scene block".into(),
            insert: "scene ${1:name} {\n    $0\n}".into(),
        },
        Vs2Completion {
            label: "say".into(),
            kind: Vs2CompletionKind::Snippet,
            detail: "say with t!".into(),
            insert: "say ${1:speaker}, t!(\"${2:key}\");".into(),
        },
        Vs2Completion {
            label: "menu".into(),
            kind: Vs2CompletionKind::Snippet,
            detail: "menu choices".into(),
            insert: "menu {\n    t!(\"${1:a}\") => { $0 }\n}".into(),
        },
        Vs2Completion {
            label: "screen".into(),
            kind: Vs2CompletionKind::Snippet,
            detail: "typed screen".into(),
            insert: "screen ${1:Name} {\n    $0\n}".into(),
        },
        Vs2Completion {
            label: "push_layer".into(),
            kind: Vs2CompletionKind::Function,
            detail: "push_layer(LayerId)".into(),
            insert: "push_layer(LayerId::new(\"${1:id}\"))?;".into(),
        },
    ]
}

pub fn default_completions() -> Vec<Vs2Completion> {
    let mut v = Vec::new();
    for k in VS2_KEYWORDS {
        v.push(Vs2Completion::kw(k));
    }
    for t in VS2_TYPES {
        v.push(Vs2Completion::ty(t));
    }
    v.extend(story_snippets());
    v
}

pub fn filter_completions(prefix: &str) -> Vec<Vs2Completion> {
    let p = prefix.to_ascii_lowercase();
    default_completions()
        .into_iter()
        .filter(|c| c.label.to_ascii_lowercase().starts_with(&p))
        .collect()
}

pub fn hover_for(name: &str) -> Option<String> {
    match name {
        "LayerId" => Some("stable layer handle — not a Python string global".into()),
        "MsgId" => Some("message key for i18n; use t!(\"key\")".into()),
        "SceneId" => Some("typed scene label for jump/call".into()),
        "say" => Some("say speaker, t!(\"key\") — dialogue line".into()),
        "push_layer" => Some("push exclusive UI layer onto stack".into()),
        "fn" => Some("function item (rust-like, not def)".into()),
        "struct" => Some("product type with named fields".into()),
        "enum" => Some("sum type with typed variants".into()),
        "match" => Some("exhaustive pattern match".into()),
        "scene" => Some("story scene / label block".into()),
        _ => None,
    }
}

pub static SEMANTIC_TYPES: &[&str] = &[
    "keyword",
    "function",
    "type",
    "variable",
    "parameter",
    "property",
    "string",
    "number",
    "comment",
    "namespace",
    "macro",
    "enumMember",
];

/// Completions for names discovered in a module (scenes, layers, msg keys).
pub fn local_completions(
    mod_name: &str,
    scenes: &[&str],
    layers: &[&str],
    msg_keys: &[&str],
) -> Vec<Vs2Completion> {
    let mut v = Vec::new();
    v.push(Vs2Completion::fn_item(
        &format!("{mod_name}_main"),
        "module entry",
    ));
    for s in scenes {
        v.push(Vs2Completion {
            label: (*s).into(),
            kind: Vs2CompletionKind::Scene,
            detail: "scene".into(),
            insert: (*s).into(),
        });
    }
    for l in layers {
        v.push(Vs2Completion {
            label: (*l).into(),
            kind: Vs2CompletionKind::Layer,
            detail: "layer".into(),
            insert: format!("LayerId::new(\"{l}\")"),
        });
    }
    for k in msg_keys {
        v.push(Vs2Completion {
            label: (*k).into(),
            kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(),
            insert: format!("t!(\"{k}\")"),
        });
    }
    v
}

pub fn classify_word(word: &str) -> &'static str {
    if VS2_KEYWORDS.contains(&word) {
        "keyword"
    } else if VS2_TYPES.contains(&word) {
        "type"
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        "type"
    } else {
        "variable"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keywords_present() {
        let c = default_completions();
        assert!(c.iter().any(|x| x.label == "fn"));
        assert!(c.iter().any(|x| x.label == "scene"));
        assert!(c.iter().any(|x| x.label == "LayerId"));
    }
    #[test]
    fn filter_fn() {
        let c = filter_completions("sc");
        assert!(c.iter().any(|x| x.label.starts_with("sc")));
    }
    #[test]
    fn hover_layer() {
        assert!(hover_for("LayerId").unwrap().contains("layer"));
    }
    #[test]
    fn classify() {
        assert_eq!(classify_word("fn"), "keyword");
        assert_eq!(classify_word("i32"), "type");
    }
}
