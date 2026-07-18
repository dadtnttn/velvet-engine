//! VS2 LSP helpers: completions, hover, semantic tokens for rust-like surface.

#![allow(missing_docs)]
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vs2CompletionKind {
    Keyword, Function, Type, Variable, Module, Snippet, Layer, Scene, MsgKey,
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
        Self { label: label.into(), kind: Vs2CompletionKind::Keyword, detail: "keyword".into(), insert: label.into() }
    }
    pub fn fn_item(label: &str, detail: &str) -> Self {
        Self { label: label.into(), kind: Vs2CompletionKind::Function, detail: detail.into(), insert: format!("{label}($0)") }
    }
    pub fn ty(label: &str) -> Self {
        Self { label: label.into(), kind: Vs2CompletionKind::Type, detail: "type".into(), insert: label.into() }
    }
}

pub static VS2_KEYWORDS: &[&str] = &[
    "fn", "struct", "enum", "mod", "use", "pub", "let", "mut", "const", "static",
    "if", "else", "while", "for", "loop", "match", "return", "break", "continue",
    "impl", "trait", "type", "where", "as", "in", "ref", "move",
    "scene", "say", "menu", "jump", "call", "show", "hide", "with", "at",
    "character", "screen", "state", "transform", "layer",
    "true", "false", "self", "Self", "crate", "super",
];

pub static VS2_TYPES: &[&str] = &[
    "i32", "i64", "u32", "u64", "f32", "f64", "bool", "str", "String",
    "Option", "Result", "Vec", "LayerId", "SceneId", "MsgId", "ScriptError",
    "Transform", "Transition", "Color", "Vec2",
];

pub fn story_snippets() -> Vec<Vs2Completion> {
    vec![
        Vs2Completion { label: "scene".into(), kind: Vs2CompletionKind::Snippet,
            detail: "scene block".into(), insert: "scene ${1:name} {\n    $0\n}".into() },
        Vs2Completion { label: "say".into(), kind: Vs2CompletionKind::Snippet,
            detail: "say with t!".into(), insert: "say ${1:speaker}, t!(\"${2:key}\");".into() },
        Vs2Completion { label: "menu".into(), kind: Vs2CompletionKind::Snippet,
            detail: "menu choices".into(), insert: "menu {\n    t!(\"${1:a}\") => { $0 }\n}".into() },
        Vs2Completion { label: "screen".into(), kind: Vs2CompletionKind::Snippet,
            detail: "typed screen".into(), insert: "screen ${1:Name} {\n    $0\n}".into() },
        Vs2Completion { label: "push_layer".into(), kind: Vs2CompletionKind::Function,
            detail: "push_layer(LayerId)".into(),
            insert: "push_layer(LayerId::new(\"${1:id}\"))?;".into() },
    ]
}

pub fn default_completions() -> Vec<Vs2Completion> {
    let mut v = Vec::new();
    for k in VS2_KEYWORDS { v.push(Vs2Completion::kw(k)); }
    for t in VS2_TYPES { v.push(Vs2Completion::ty(t)); }
    v.extend(story_snippets());
    v
}

pub fn filter_completions(prefix: &str) -> Vec<Vs2Completion> {
    let p = prefix.to_ascii_lowercase();
    default_completions().into_iter()
        .filter(|c| c.label.to_ascii_lowercase().starts_with(&p)).collect()
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
    "keyword", "function", "type", "variable", "parameter", "property",
    "string", "number", "comment", "namespace", "macro", "enumMember",
];

pub fn classify_word(word: &str) -> &'static str {
    if VS2_KEYWORDS.contains(&word) { "keyword" }
    else if VS2_TYPES.contains(&word) { "type" }
    else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) { "type" }
    else { "variable" }
}

pub fn local_completions_0(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_0"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_0")),
        Vs2Completion { label: format!("scene_0"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_0") },
        Vs2Completion { label: format!("layer_0"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_0\")") },
        Vs2Completion { label: format!("msg.k0"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k0\")") },
    ]
}

pub fn local_completions_1(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_1"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_1")),
        Vs2Completion { label: format!("scene_1"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_1") },
        Vs2Completion { label: format!("layer_1"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_1\")") },
        Vs2Completion { label: format!("msg.k1"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k1\")") },
    ]
}

pub fn local_completions_2(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_2"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_2")),
        Vs2Completion { label: format!("scene_2"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_2") },
        Vs2Completion { label: format!("layer_2"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_2\")") },
        Vs2Completion { label: format!("msg.k2"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k2\")") },
    ]
}

pub fn local_completions_3(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_3"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_3")),
        Vs2Completion { label: format!("scene_3"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_3") },
        Vs2Completion { label: format!("layer_3"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_3\")") },
        Vs2Completion { label: format!("msg.k3"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k3\")") },
    ]
}

pub fn local_completions_4(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_4"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_4")),
        Vs2Completion { label: format!("scene_4"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_4") },
        Vs2Completion { label: format!("layer_4"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_4\")") },
        Vs2Completion { label: format!("msg.k4"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k4\")") },
    ]
}

pub fn local_completions_5(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_5"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_5")),
        Vs2Completion { label: format!("scene_5"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_5") },
        Vs2Completion { label: format!("layer_5"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_5\")") },
        Vs2Completion { label: format!("msg.k5"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k5\")") },
    ]
}

pub fn local_completions_6(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_6"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_6")),
        Vs2Completion { label: format!("scene_6"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_6") },
        Vs2Completion { label: format!("layer_6"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_6\")") },
        Vs2Completion { label: format!("msg.k6"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k6\")") },
    ]
}

pub fn local_completions_7(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_7"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_7")),
        Vs2Completion { label: format!("scene_7"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_7") },
        Vs2Completion { label: format!("layer_7"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_7\")") },
        Vs2Completion { label: format!("msg.k7"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k7\")") },
    ]
}

pub fn local_completions_8(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_8"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_8")),
        Vs2Completion { label: format!("scene_8"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_8") },
        Vs2Completion { label: format!("layer_8"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_8\")") },
        Vs2Completion { label: format!("msg.k8"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k8\")") },
    ]
}

pub fn local_completions_9(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_9"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_9")),
        Vs2Completion { label: format!("scene_9"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_9") },
        Vs2Completion { label: format!("layer_9"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_9\")") },
        Vs2Completion { label: format!("msg.k9"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k9\")") },
    ]
}

pub fn local_completions_10(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_10"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_10")),
        Vs2Completion { label: format!("scene_10"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_10") },
        Vs2Completion { label: format!("layer_10"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_10\")") },
        Vs2Completion { label: format!("msg.k10"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k10\")") },
    ]
}

pub fn local_completions_11(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_11"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_11")),
        Vs2Completion { label: format!("scene_11"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_11") },
        Vs2Completion { label: format!("layer_11"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_11\")") },
        Vs2Completion { label: format!("msg.k11"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k11\")") },
    ]
}

pub fn local_completions_12(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_12"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_12")),
        Vs2Completion { label: format!("scene_12"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_12") },
        Vs2Completion { label: format!("layer_12"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_12\")") },
        Vs2Completion { label: format!("msg.k12"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k12\")") },
    ]
}

pub fn local_completions_13(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_13"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_13")),
        Vs2Completion { label: format!("scene_13"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_13") },
        Vs2Completion { label: format!("layer_13"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_13\")") },
        Vs2Completion { label: format!("msg.k13"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k13\")") },
    ]
}

pub fn local_completions_14(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_14"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_14")),
        Vs2Completion { label: format!("scene_14"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_14") },
        Vs2Completion { label: format!("layer_14"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_14\")") },
        Vs2Completion { label: format!("msg.k14"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k14\")") },
    ]
}

pub fn local_completions_15(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_15"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_15")),
        Vs2Completion { label: format!("scene_15"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_15") },
        Vs2Completion { label: format!("layer_15"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_15\")") },
        Vs2Completion { label: format!("msg.k15"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k15\")") },
    ]
}

pub fn local_completions_16(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_16"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_16")),
        Vs2Completion { label: format!("scene_16"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_16") },
        Vs2Completion { label: format!("layer_16"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_16\")") },
        Vs2Completion { label: format!("msg.k16"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k16\")") },
    ]
}

pub fn local_completions_17(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_17"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_17")),
        Vs2Completion { label: format!("scene_17"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_17") },
        Vs2Completion { label: format!("layer_17"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_17\")") },
        Vs2Completion { label: format!("msg.k17"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k17\")") },
    ]
}

pub fn local_completions_18(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_18"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_18")),
        Vs2Completion { label: format!("scene_18"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_18") },
        Vs2Completion { label: format!("layer_18"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_18\")") },
        Vs2Completion { label: format!("msg.k18"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k18\")") },
    ]
}

pub fn local_completions_19(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_19"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_19")),
        Vs2Completion { label: format!("scene_19"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_19") },
        Vs2Completion { label: format!("layer_19"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_19\")") },
        Vs2Completion { label: format!("msg.k19"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k19\")") },
    ]
}

pub fn local_completions_20(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_20"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_20")),
        Vs2Completion { label: format!("scene_20"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_20") },
        Vs2Completion { label: format!("layer_20"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_20\")") },
        Vs2Completion { label: format!("msg.k20"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k20\")") },
    ]
}

pub fn local_completions_21(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_21"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_21")),
        Vs2Completion { label: format!("scene_21"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_21") },
        Vs2Completion { label: format!("layer_21"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_21\")") },
        Vs2Completion { label: format!("msg.k21"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k21\")") },
    ]
}

pub fn local_completions_22(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_22"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_22")),
        Vs2Completion { label: format!("scene_22"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_22") },
        Vs2Completion { label: format!("layer_22"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_22\")") },
        Vs2Completion { label: format!("msg.k22"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k22\")") },
    ]
}

pub fn local_completions_23(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_23"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_23")),
        Vs2Completion { label: format!("scene_23"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_23") },
        Vs2Completion { label: format!("layer_23"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_23\")") },
        Vs2Completion { label: format!("msg.k23"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k23\")") },
    ]
}

pub fn local_completions_24(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_24"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_24")),
        Vs2Completion { label: format!("scene_24"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_24") },
        Vs2Completion { label: format!("layer_24"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_24\")") },
        Vs2Completion { label: format!("msg.k24"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k24\")") },
    ]
}

pub fn local_completions_25(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_25"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_25")),
        Vs2Completion { label: format!("scene_25"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_25") },
        Vs2Completion { label: format!("layer_25"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_25\")") },
        Vs2Completion { label: format!("msg.k25"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k25\")") },
    ]
}

pub fn local_completions_26(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_26"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_26")),
        Vs2Completion { label: format!("scene_26"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_26") },
        Vs2Completion { label: format!("layer_26"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_26\")") },
        Vs2Completion { label: format!("msg.k26"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k26\")") },
    ]
}

pub fn local_completions_27(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_27"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_27")),
        Vs2Completion { label: format!("scene_27"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_27") },
        Vs2Completion { label: format!("layer_27"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_27\")") },
        Vs2Completion { label: format!("msg.k27"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k27\")") },
    ]
}

pub fn local_completions_28(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_28"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_28")),
        Vs2Completion { label: format!("scene_28"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_28") },
        Vs2Completion { label: format!("layer_28"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_28\")") },
        Vs2Completion { label: format!("msg.k28"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k28\")") },
    ]
}

pub fn local_completions_29(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_29"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_29")),
        Vs2Completion { label: format!("scene_29"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_29") },
        Vs2Completion { label: format!("layer_29"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_29\")") },
        Vs2Completion { label: format!("msg.k29"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k29\")") },
    ]
}

pub fn local_completions_30(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_30"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_30")),
        Vs2Completion { label: format!("scene_30"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_30") },
        Vs2Completion { label: format!("layer_30"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_30\")") },
        Vs2Completion { label: format!("msg.k30"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k30\")") },
    ]
}

pub fn local_completions_31(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_31"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_31")),
        Vs2Completion { label: format!("scene_31"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_31") },
        Vs2Completion { label: format!("layer_31"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_31\")") },
        Vs2Completion { label: format!("msg.k31"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k31\")") },
    ]
}

pub fn local_completions_32(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_32"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_32")),
        Vs2Completion { label: format!("scene_32"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_32") },
        Vs2Completion { label: format!("layer_32"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_32\")") },
        Vs2Completion { label: format!("msg.k32"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k32\")") },
    ]
}

pub fn local_completions_33(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_33"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_33")),
        Vs2Completion { label: format!("scene_33"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_33") },
        Vs2Completion { label: format!("layer_33"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_33\")") },
        Vs2Completion { label: format!("msg.k33"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k33\")") },
    ]
}

pub fn local_completions_34(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_34"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_34")),
        Vs2Completion { label: format!("scene_34"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_34") },
        Vs2Completion { label: format!("layer_34"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_34\")") },
        Vs2Completion { label: format!("msg.k34"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k34\")") },
    ]
}

pub fn local_completions_35(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_35"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_35")),
        Vs2Completion { label: format!("scene_35"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_35") },
        Vs2Completion { label: format!("layer_35"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_35\")") },
        Vs2Completion { label: format!("msg.k35"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k35\")") },
    ]
}

pub fn local_completions_36(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_36"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_36")),
        Vs2Completion { label: format!("scene_36"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_36") },
        Vs2Completion { label: format!("layer_36"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_36\")") },
        Vs2Completion { label: format!("msg.k36"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k36\")") },
    ]
}

pub fn local_completions_37(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_37"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_37")),
        Vs2Completion { label: format!("scene_37"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_37") },
        Vs2Completion { label: format!("layer_37"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_37\")") },
        Vs2Completion { label: format!("msg.k37"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k37\")") },
    ]
}

pub fn local_completions_38(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_38"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_38")),
        Vs2Completion { label: format!("scene_38"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_38") },
        Vs2Completion { label: format!("layer_38"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_38\")") },
        Vs2Completion { label: format!("msg.k38"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k38\")") },
    ]
}

pub fn local_completions_39(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_39"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_39")),
        Vs2Completion { label: format!("scene_39"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_39") },
        Vs2Completion { label: format!("layer_39"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_39\")") },
        Vs2Completion { label: format!("msg.k39"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k39\")") },
    ]
}

pub fn local_completions_40(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_40"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_40")),
        Vs2Completion { label: format!("scene_40"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_40") },
        Vs2Completion { label: format!("layer_40"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_40\")") },
        Vs2Completion { label: format!("msg.k40"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k40\")") },
    ]
}

pub fn local_completions_41(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_41"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_41")),
        Vs2Completion { label: format!("scene_41"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_41") },
        Vs2Completion { label: format!("layer_41"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_41\")") },
        Vs2Completion { label: format!("msg.k41"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k41\")") },
    ]
}

pub fn local_completions_42(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_42"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_42")),
        Vs2Completion { label: format!("scene_42"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_42") },
        Vs2Completion { label: format!("layer_42"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_42\")") },
        Vs2Completion { label: format!("msg.k42"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k42\")") },
    ]
}

pub fn local_completions_43(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_43"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_43")),
        Vs2Completion { label: format!("scene_43"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_43") },
        Vs2Completion { label: format!("layer_43"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_43\")") },
        Vs2Completion { label: format!("msg.k43"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k43\")") },
    ]
}

pub fn local_completions_44(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_44"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_44")),
        Vs2Completion { label: format!("scene_44"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_44") },
        Vs2Completion { label: format!("layer_44"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_44\")") },
        Vs2Completion { label: format!("msg.k44"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k44\")") },
    ]
}

pub fn local_completions_45(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_45"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_45")),
        Vs2Completion { label: format!("scene_45"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_45") },
        Vs2Completion { label: format!("layer_45"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_45\")") },
        Vs2Completion { label: format!("msg.k45"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k45\")") },
    ]
}

pub fn local_completions_46(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_46"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_46")),
        Vs2Completion { label: format!("scene_46"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_46") },
        Vs2Completion { label: format!("layer_46"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_46\")") },
        Vs2Completion { label: format!("msg.k46"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k46\")") },
    ]
}

pub fn local_completions_47(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_47"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_47")),
        Vs2Completion { label: format!("scene_47"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_47") },
        Vs2Completion { label: format!("layer_47"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_47\")") },
        Vs2Completion { label: format!("msg.k47"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k47\")") },
    ]
}

pub fn local_completions_48(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_48"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_48")),
        Vs2Completion { label: format!("scene_48"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_48") },
        Vs2Completion { label: format!("layer_48"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_48\")") },
        Vs2Completion { label: format!("msg.k48"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k48\")") },
    ]
}

pub fn local_completions_49(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_49"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_49")),
        Vs2Completion { label: format!("scene_49"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_49") },
        Vs2Completion { label: format!("layer_49"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_49\")") },
        Vs2Completion { label: format!("msg.k49"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k49\")") },
    ]
}

pub fn local_completions_50(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_50"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_50")),
        Vs2Completion { label: format!("scene_50"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_50") },
        Vs2Completion { label: format!("layer_50"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_50\")") },
        Vs2Completion { label: format!("msg.k50"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k50\")") },
    ]
}

pub fn local_completions_51(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_51"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_51")),
        Vs2Completion { label: format!("scene_51"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_51") },
        Vs2Completion { label: format!("layer_51"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_51\")") },
        Vs2Completion { label: format!("msg.k51"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k51\")") },
    ]
}

pub fn local_completions_52(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_52"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_52")),
        Vs2Completion { label: format!("scene_52"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_52") },
        Vs2Completion { label: format!("layer_52"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_52\")") },
        Vs2Completion { label: format!("msg.k52"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k52\")") },
    ]
}

pub fn local_completions_53(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_53"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_53")),
        Vs2Completion { label: format!("scene_53"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_53") },
        Vs2Completion { label: format!("layer_53"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_53\")") },
        Vs2Completion { label: format!("msg.k53"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k53\")") },
    ]
}

pub fn local_completions_54(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_54"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_54")),
        Vs2Completion { label: format!("scene_54"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_54") },
        Vs2Completion { label: format!("layer_54"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_54\")") },
        Vs2Completion { label: format!("msg.k54"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k54\")") },
    ]
}

pub fn local_completions_55(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_55"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_55")),
        Vs2Completion { label: format!("scene_55"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_55") },
        Vs2Completion { label: format!("layer_55"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_55\")") },
        Vs2Completion { label: format!("msg.k55"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k55\")") },
    ]
}

pub fn local_completions_56(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_56"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_56")),
        Vs2Completion { label: format!("scene_56"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_56") },
        Vs2Completion { label: format!("layer_56"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_56\")") },
        Vs2Completion { label: format!("msg.k56"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k56\")") },
    ]
}

pub fn local_completions_57(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_57"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_57")),
        Vs2Completion { label: format!("scene_57"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_57") },
        Vs2Completion { label: format!("layer_57"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_57\")") },
        Vs2Completion { label: format!("msg.k57"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k57\")") },
    ]
}

pub fn local_completions_58(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_58"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_58")),
        Vs2Completion { label: format!("scene_58"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_58") },
        Vs2Completion { label: format!("layer_58"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_58\")") },
        Vs2Completion { label: format!("msg.k58"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k58\")") },
    ]
}

pub fn local_completions_59(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_59"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_59")),
        Vs2Completion { label: format!("scene_59"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_59") },
        Vs2Completion { label: format!("layer_59"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_59\")") },
        Vs2Completion { label: format!("msg.k59"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k59\")") },
    ]
}

pub fn local_completions_60(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_60"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_60")),
        Vs2Completion { label: format!("scene_60"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_60") },
        Vs2Completion { label: format!("layer_60"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_60\")") },
        Vs2Completion { label: format!("msg.k60"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k60\")") },
    ]
}

pub fn local_completions_61(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_61"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_61")),
        Vs2Completion { label: format!("scene_61"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_61") },
        Vs2Completion { label: format!("layer_61"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_61\")") },
        Vs2Completion { label: format!("msg.k61"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k61\")") },
    ]
}

pub fn local_completions_62(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_62"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_62")),
        Vs2Completion { label: format!("scene_62"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_62") },
        Vs2Completion { label: format!("layer_62"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_62\")") },
        Vs2Completion { label: format!("msg.k62"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k62\")") },
    ]
}

pub fn local_completions_63(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_63"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_63")),
        Vs2Completion { label: format!("scene_63"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_63") },
        Vs2Completion { label: format!("layer_63"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_63\")") },
        Vs2Completion { label: format!("msg.k63"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k63\")") },
    ]
}

pub fn local_completions_64(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_64"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_64")),
        Vs2Completion { label: format!("scene_64"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_64") },
        Vs2Completion { label: format!("layer_64"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_64\")") },
        Vs2Completion { label: format!("msg.k64"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k64\")") },
    ]
}

pub fn local_completions_65(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_65"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_65")),
        Vs2Completion { label: format!("scene_65"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_65") },
        Vs2Completion { label: format!("layer_65"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_65\")") },
        Vs2Completion { label: format!("msg.k65"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k65\")") },
    ]
}

pub fn local_completions_66(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_66"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_66")),
        Vs2Completion { label: format!("scene_66"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_66") },
        Vs2Completion { label: format!("layer_66"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_66\")") },
        Vs2Completion { label: format!("msg.k66"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k66\")") },
    ]
}

pub fn local_completions_67(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_67"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_67")),
        Vs2Completion { label: format!("scene_67"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_67") },
        Vs2Completion { label: format!("layer_67"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_67\")") },
        Vs2Completion { label: format!("msg.k67"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k67\")") },
    ]
}

pub fn local_completions_68(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_68"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_68")),
        Vs2Completion { label: format!("scene_68"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_68") },
        Vs2Completion { label: format!("layer_68"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_68\")") },
        Vs2Completion { label: format!("msg.k68"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k68\")") },
    ]
}

pub fn local_completions_69(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_69"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_69")),
        Vs2Completion { label: format!("scene_69"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_69") },
        Vs2Completion { label: format!("layer_69"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_69\")") },
        Vs2Completion { label: format!("msg.k69"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k69\")") },
    ]
}

pub fn local_completions_70(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_70"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_70")),
        Vs2Completion { label: format!("scene_70"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_70") },
        Vs2Completion { label: format!("layer_70"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_70\")") },
        Vs2Completion { label: format!("msg.k70"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k70\")") },
    ]
}

pub fn local_completions_71(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_71"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_71")),
        Vs2Completion { label: format!("scene_71"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_71") },
        Vs2Completion { label: format!("layer_71"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_71\")") },
        Vs2Completion { label: format!("msg.k71"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k71\")") },
    ]
}

pub fn local_completions_72(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_72"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_72")),
        Vs2Completion { label: format!("scene_72"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_72") },
        Vs2Completion { label: format!("layer_72"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_72\")") },
        Vs2Completion { label: format!("msg.k72"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k72\")") },
    ]
}

pub fn local_completions_73(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_73"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_73")),
        Vs2Completion { label: format!("scene_73"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_73") },
        Vs2Completion { label: format!("layer_73"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_73\")") },
        Vs2Completion { label: format!("msg.k73"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k73\")") },
    ]
}

pub fn local_completions_74(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_74"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_74")),
        Vs2Completion { label: format!("scene_74"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_74") },
        Vs2Completion { label: format!("layer_74"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_74\")") },
        Vs2Completion { label: format!("msg.k74"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k74\")") },
    ]
}

pub fn local_completions_75(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_75"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_75")),
        Vs2Completion { label: format!("scene_75"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_75") },
        Vs2Completion { label: format!("layer_75"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_75\")") },
        Vs2Completion { label: format!("msg.k75"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k75\")") },
    ]
}

pub fn local_completions_76(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_76"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_76")),
        Vs2Completion { label: format!("scene_76"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_76") },
        Vs2Completion { label: format!("layer_76"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_76\")") },
        Vs2Completion { label: format!("msg.k76"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k76\")") },
    ]
}

pub fn local_completions_77(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_77"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_77")),
        Vs2Completion { label: format!("scene_77"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_77") },
        Vs2Completion { label: format!("layer_77"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_77\")") },
        Vs2Completion { label: format!("msg.k77"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k77\")") },
    ]
}

pub fn local_completions_78(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_78"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_78")),
        Vs2Completion { label: format!("scene_78"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_78") },
        Vs2Completion { label: format!("layer_78"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_78\")") },
        Vs2Completion { label: format!("msg.k78"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k78\")") },
    ]
}

pub fn local_completions_79(mod_name: &str) -> Vec<Vs2Completion> {
    vec![
        Vs2Completion::fn_item(&format!("{mod_name}_fn_79"), "local fn"),
        Vs2Completion::ty(&format!("{mod_name}_Ty_79")),
        Vs2Completion { label: format!("scene_79"), kind: Vs2CompletionKind::Scene,
            detail: "scene".into(), insert: format!("scene_79") },
        Vs2Completion { label: format!("layer_79"), kind: Vs2CompletionKind::Layer,
            detail: "layer".into(), insert: format!("LayerId::new(\"layer_79\")") },
        Vs2Completion { label: format!("msg.k79"), kind: Vs2CompletionKind::MsgKey,
            detail: "msg".into(), insert: format!("t!(\"msg.k79\")") },
    ]
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
    fn hover_layer() { assert!(hover_for("LayerId").unwrap().contains("layer")); }
    #[test]
    fn classify() {
        assert_eq!(classify_word("fn"), "keyword");
        assert_eq!(classify_word("i32"), "type");
    }
}

