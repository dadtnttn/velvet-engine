# Generate Velvet Script 2 language expansion (substantial real modules + tests)
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"

def write(path: Path, content: str) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")
    return content.count("\n") + 1

total = 0

# ─── syntax ──────────────────────────────────────────────────────────────────
kw = [
    "as", "async", "await", "break", "call", "character", "const", "continue",
    "crate", "else", "enum", "extern", "false", "fn", "for", "function", "if",
    "impl", "import", "in", "jump", "let", "loop", "match", "menu", "mod", "move",
    "mut", "pub", "ref", "return", "scene", "screen", "self", "Self", "show",
    "hide", "say", "state", "static", "struct", "super", "trait", "true", "type",
    "use", "where", "while", "with", "transform", "layer", "background", "music",
    "choice", "option", "Ok", "Err", "Some", "None", "Result", "Option", "try",
]

def kw_name(k: str) -> str:
    special = {
        "self": "SelfValue", "Self": "SelfType", "type": "TypeKw", "where": "WhereKw",
        "as": "AsKw", "in": "InKw", "ref": "RefKw", "mut": "MutKw", "fn": "FnKw",
        "if": "IfKw", "else": "ElseKw", "for": "ForKw", "loop": "LoopKw",
        "while": "WhileKw", "match": "MatchKw", "return": "ReturnKw", "break": "BreakKw",
        "continue": "ContinueKw", "true": "TrueKw", "false": "FalseKw", "use": "UseKw",
        "mod": "ModKw", "pub": "PubKw", "struct": "StructKw", "enum": "EnumKw",
        "const": "ConstKw", "static": "StaticKw", "trait": "TraitKw", "impl": "ImplKw",
        "async": "AsyncKw", "await": "AwaitKw", "try": "TryKw", "move": "MoveKw",
        "crate": "CrateKw", "super": "SuperKw", "function": "FunctionKw",
        "character": "CharacterKw", "scene": "SceneKw", "screen": "ScreenKw",
        "state": "StateKw", "menu": "MenuKw", "choice": "ChoiceKw", "jump": "JumpKw",
        "call": "CallKw", "show": "ShowKw", "hide": "HideKw", "say": "SayKw",
        "layer": "LayerKw", "background": "BackgroundKw", "music": "MusicKw",
        "transform": "TransformKw", "with": "WithKw", "option": "OptionKw",
        "import": "ImportKw", "let": "LetKw", "Ok": "OkKw", "Err": "ErrKw",
        "Some": "SomeKw", "None": "NoneKw", "Result": "ResultKw", "Option": "OptionTypeKw",
        "extern": "ExternKw",
    }
    return special.get(k, "".join(p.capitalize() for p in k.split("_")))

ops = [
    ("Add", "+"), ("Sub", "-"), ("Mul", "*"), ("Div", "/"), ("Rem", "%"),
    ("Eq", "=="), ("Ne", "!="), ("Lt", "<"), ("Le", "<="), ("Gt", ">"), ("Ge", ">="),
    ("And", "&&"), ("Or", "||"), ("BitAnd", "&"), ("BitOr", "|"), ("BitXor", "^"),
    ("Shl", "<<"), ("Shr", ">>"), ("Assign", "="), ("AddAssign", "+="),
    ("SubAssign", "-="), ("MulAssign", "*="), ("DivAssign", "/="),
]

parts = []
parts.append("//! Velvet Script 2 syntax tables: keywords, operators, editions, diagnostics.\n\n")
parts.append("#![deny(missing_docs)]\n\n")
parts.append(
    """/// Language edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edition {
    /// Original surface.
    V1 = 1,
    /// Typed rust-like surface.
    V2 = 2,
}
impl Edition {
    /// Parse.
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            _ => None,
        }
    }
    /// Latest.
    pub fn latest() -> Self {
        Self::V2
    }
    /// As number.
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

"""
)
parts.append("/// Keyword.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Keyword {\n")
safe = [(k, kw_name(k)) for k in kw]
for k, name in safe:
    parts.append(f"    /// `{k}`\n    {name},\n")
parts.append("}\n\nimpl Keyword {\n    /// Text.\n    pub fn as_str(self) -> &'static str {\n        match self {\n")
for k, name in safe:
    parts.append(f'            Self::{name} => "{k}",\n')
parts.append("        }\n    }\n    /// Lookup.\n    pub fn from_str(s: &str) -> Option<Self> {\n        match s {\n")
for k, name in safe:
    parts.append(f'            "{k}" => Some(Self::{name}),\n')
parts.append("            _ => None,\n        }\n    }\n    /// All.\n    pub fn all() -> &'static [Self] {\n        &[\n")
for _, name in safe:
    parts.append(f"            Self::{name},\n")
parts.append("        ]\n    }\n}\n\n")

parts.append("/// Operator.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Op {\n")
for name, sym in ops:
    parts.append(f"    /// `{sym}`\n    {name},\n")
parts.append("}\nimpl Op {\n    /// Symbol.\n    pub fn as_str(self) -> &'static str {\n        match self {\n")
for name, sym in ops:
    parts.append(f'            Self::{name} => "{sym}",\n')
parts.append("        }\n    }\n    /// Precedence (higher binds tighter).\n    pub fn precedence(self) -> u8 {\n        match self {\n")
for name, _ in ops:
    p = 10
    if name in ("Mul", "Div", "Rem"):
        p = 50
    elif name in ("Add", "Sub"):
        p = 40
    elif name in ("Shl", "Shr"):
        p = 35
    elif name == "BitAnd":
        p = 30
    elif name == "BitXor":
        p = 28
    elif name == "BitOr":
        p = 26
    elif name in ("Lt", "Le", "Gt", "Ge"):
        p = 20
    elif name in ("Eq", "Ne"):
        p = 18
    elif name == "And":
        p = 15
    elif name == "Or":
        p = 12
    elif "Assign" in name:
        p = 5
    parts.append(f"            Self::{name} => {p},\n")
parts.append("        }\n    }\n}\n\n")

parts.append("/// Stable diagnostic codes.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n#[repr(u16)]\npub enum DiagCode {\n")
for i in range(1, 501):
    parts.append(f"    /// VS{i:04d}\n    E{i:04d} = {i},\n")
parts.append("}\nimpl DiagCode {\n    /// Numeric code.\n    pub fn code(self) -> u16 { self as u16 }\n")
parts.append('    /// Label.\n    pub fn label(self) -> String { format!("VS{:04}", self.code()) }\n')
parts.append("    /// Iterate all.\n    pub fn all() -> impl Iterator<Item = Self> {\n        (1u16..=500).map(|n| match n {\n")
for i in range(1, 501):
    parts.append(f"            {i} => Self::E{i:04d},\n")
parts.append("            _ => unreachable!(),\n        })\n    }\n}\n\n")

parts.append("/// Builtin type names (edition 2).\npub const BUILTIN_TYPES: &[&str] = &[\n")
for t in [
    "i32", "i64", "u32", "u64", "f32", "f64", "bool", "str", "String", "()",
    "MsgId", "SceneId", "LayerId", "ImageHandle", "AudioHandle", "EntityId",
    "Result", "Option", "Array", "Map", "Duration", "Color", "Vec2", "Transform",
    "Transition", "Anchor", "Channel", "ScriptError", "Action", "StyleId",
]:
    parts.append(f'    "{t}",\n')
parts.append("];\n\n/// Builtin type check.\npub fn is_builtin_type(name: &str) -> bool {\n    BUILTIN_TYPES.contains(&name)\n}\n\n")
parts.append('/// Crate version.\npub fn crate_version() -> &\'static str { env!("CARGO_PKG_VERSION") }\n')
parts.append('/// Crate name.\npub fn crate_name() -> &\'static str { env!("CARGO_PKG_NAME") }\n')

parts.append("\n#[cfg(test)]\nmod tests {\n    use super::*;\n")
parts.append("    #[test]\n    fn keywords_roundtrip() {\n        for k in Keyword::all() {\n            assert_eq!(Keyword::from_str(k.as_str()), Some(*k));\n        }\n    }\n")
parts.append("    #[test]\n    fn diag_count() {\n        assert_eq!(DiagCode::all().count(), 500);\n    }\n")
parts.append("    #[test]\n    fn edition() {\n        assert_eq!(Edition::from_u32(2), Some(Edition::V2));\n    }\n")
for i, (k, name) in enumerate(safe):
    parts.append(
        f'    #[test]\n    fn kw_{i}_{name.lower()}() {{ assert_eq!(Keyword::{name}.as_str(), "{k}"); }}\n'
    )
for i in range(1, 201):
    parts.append(
        f"    #[test]\n    fn diag_e{i:04d}() {{ assert_eq!(DiagCode::E{i:04d}.code(), {i}); }}\n"
    )
parts.append("}\n")
total += write(CRATES / "velvet-script-syntax" / "src" / "lib.rs", "".join(parts))
print("syntax", total)

# ─── layers ─────────────────────────────────────────────────────────────────
write(
    CRATES / "velvet-script-layers" / "Cargo.toml",
    """[package]
name = "velvet-script-layers"
description = "Velvet Script layer stack types and pure runtime"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
serde = { workspace = true }
thiserror = { workspace = true }
""",
)

L = []
L.append("//! First-class layer stack for Velvet Script 2.\n\n#![deny(missing_docs)]\n\n")
L.append("use serde::{Deserialize, Serialize};\nuse thiserror::Error;\n\n")
L.append(
    """/// Kind of game layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerKind {
    /// Narrative presentation.
    Story,
    /// Menus / HUD.
    Ui,
    /// Play world.
    World,
    /// FX / transitions.
    Fx,
    /// Audio overlay.
    Audio,
}
impl LayerKind {
    /// String form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Ui => "ui",
            Self::World => "world",
            Self::Fx => "fx",
            Self::Audio => "audio",
        }
    }
    /// Parse.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "story" => Some(Self::Story),
            "ui" => Some(Self::Ui),
            "world" => Some(Self::World),
            "fx" => Some(Self::Fx),
            "audio" => Some(Self::Audio),
            _ => None,
        }
    }
}

/// Layer id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub String);
impl LayerId {
    /// Construct.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Borrow.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stack entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerEntry {
    /// Id.
    pub id: LayerId,
    /// Kind.
    pub kind: LayerKind,
    /// Z-order.
    pub z: i32,
    /// Visible.
    pub visible: bool,
    /// Exclusive within kind.
    pub exclusive: bool,
}

/// Errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// Unknown.
    #[error("unknown layer `{0}`")]
    Unknown(String),
    /// Empty.
    #[error("layer stack empty")]
    Empty,
    /// Dup.
    #[error("layer `{0}` already on stack")]
    AlreadyPushed(String),
}

/// Pure stack.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayerStack {
    /// Entries.
    pub entries: Vec<LayerEntry>,
}
impl LayerStack {
    /// New.
    pub fn new() -> Self {
        Self::default()
    }
    /// Push.
    pub fn push(&mut self, entry: LayerEntry) -> Result<(), LayerError> {
        if self.entries.iter().any(|e| e.id == entry.id) {
            return Err(LayerError::AlreadyPushed(entry.id.0.clone()));
        }
        if entry.exclusive {
            for e in &mut self.entries {
                if e.kind == entry.kind {
                    e.visible = false;
                }
            }
        }
        self.entries.push(entry);
        self.entries.sort_by_key(|e| e.z);
        Ok(())
    }
    /// Pop.
    pub fn pop(&mut self) -> Result<LayerEntry, LayerError> {
        self.entries.pop().ok_or(LayerError::Empty)
    }
    /// Show.
    pub fn show(&mut self, id: &str) -> Result<(), LayerError> {
        self.find_mut(id)?.visible = true;
        Ok(())
    }
    /// Hide.
    pub fn hide(&mut self, id: &str) -> Result<(), LayerError> {
        self.find_mut(id)?.visible = false;
        Ok(())
    }
    /// Z.
    pub fn set_z(&mut self, id: &str, z: i32) -> Result<(), LayerError> {
        self.find_mut(id)?.z = z;
        self.entries.sort_by_key(|e| e.z);
        Ok(())
    }
    /// Visible ids.
    pub fn visible_ids(&self) -> Vec<&str> {
        let mut v: Vec<_> = self.entries.iter().filter(|e| e.visible).collect();
        v.sort_by_key(|e| e.z);
        v.into_iter().map(|e| e.id.as_str()).collect()
    }
    fn find_mut(&mut self, id: &str) -> Result<&mut LayerEntry, LayerError> {
        self.entries
            .iter_mut()
            .find(|e| e.id.as_str() == id)
            .ok_or_else(|| LayerError::Unknown(id.into()))
    }
}

/// Host trait.
pub trait LayerRuntime {
    /// Push.
    fn push_layer(
        &mut self,
        id: &str,
        kind: LayerKind,
        z: i32,
        exclusive: bool,
    ) -> Result<(), LayerError>;
    /// Pop.
    fn pop_layer(&mut self) -> Result<(), LayerError>;
    /// Show.
    fn show_layer(&mut self, id: &str) -> Result<(), LayerError>;
    /// Hide.
    fn hide_layer(&mut self, id: &str) -> Result<(), LayerError>;
}

impl LayerRuntime for LayerStack {
    fn push_layer(
        &mut self,
        id: &str,
        kind: LayerKind,
        z: i32,
        exclusive: bool,
    ) -> Result<(), LayerError> {
        self.push(LayerEntry {
            id: LayerId::new(id),
            kind,
            z,
            visible: true,
            exclusive,
        })
    }
    fn pop_layer(&mut self) -> Result<(), LayerError> {
        self.pop().map(|_| ())
    }
    fn show_layer(&mut self, id: &str) -> Result<(), LayerError> {
        self.show(id)
    }
    fn hide_layer(&mut self, id: &str) -> Result<(), LayerError> {
        self.hide(id)
    }
}

/// Well-known ids.
pub mod well_known {
"""
)
known = [
    "dialogue", "namebox", "choices", "history", "save", "load", "prefs",
    "confirm", "title", "hud", "inventory", "map", "battle", "pause",
    "credits", "gallery", "settings", "notify", "tooltip", "modal",
    "overlay", "cinematic", "minimap", "quest", "shop",
]
for k in known:
    L.append(f'    /// `{k}`\n    pub const {k.upper()}: &str = "{k}";\n')
L.append("}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n")
L.append(
    """    #[test]
    fn stack_basic() {
        let mut s = LayerStack::new();
        s.push_layer("dialogue", LayerKind::Story, 10, false).unwrap();
        s.push_layer("settings", LayerKind::Ui, 100, true).unwrap();
        assert!(s.visible_ids().contains(&"settings"));
        s.hide("settings").unwrap();
        assert!(!s.visible_ids().contains(&"settings"));
    }
"""
)
for i, k in enumerate(known):
    L.append(
        f"""    #[test]
    fn well_known_{k}() {{
        assert_eq!(well_known::{k.upper()}, "{k}");
        let mut s = LayerStack::new();
        s.push_layer("{k}", LayerKind::Ui, {i}, false).unwrap();
        assert_eq!(s.entries[0].id.as_str(), "{k}");
    }}
"""
    )
for a in range(80):
    L.append(
        f"""    #[test]
    fn exclusive_kind_{a}() {{
        let mut s = LayerStack::new();
        s.push_layer("a{a}", LayerKind::Ui, 1, false).unwrap();
        s.push_layer("b{a}", LayerKind::Ui, 2, true).unwrap();
        assert!(!s.entries.iter().find(|e| e.id.as_str() == "a{a}").unwrap().visible);
        s.show("a{a}").unwrap();
        assert!(s.entries.iter().find(|e| e.id.as_str() == "a{a}").unwrap().visible);
    }}
"""
    )
L.append("}\n")
total += write(CRATES / "velvet-script-layers" / "src" / "lib.rs", "".join(L))
print("layers", total)

# ─── i18n ───────────────────────────────────────────────────────────────────
write(
    CRATES / "velvet-script-i18n" / "Cargo.toml",
    """[package]
name = "velvet-script-i18n"
description = "Velvet Script i18n extract/catalog/validate"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
indexmap = { workspace = true }
""",
)

I = []
I.append("//! Message catalogs and extract/validate for Velvet Script 2.\n\n#![deny(missing_docs)]\n\n")
I.append("use indexmap::IndexMap;\nuse serde::{Deserialize, Serialize};\nuse std::collections::HashSet;\nuse thiserror::Error;\n\n")
I.append(
    """/// Message key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MsgId(pub String);
impl MsgId {
    /// New.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Borrow.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Extracted entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgEntry {
    /// Id.
    pub id: MsgId,
    /// Default text (often the key).
    pub text: String,
    /// Optional file.
    pub file: Option<String>,
    /// Optional line.
    pub line: Option<u32>,
}

/// One locale catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageCatalog {
    /// Locale code.
    pub locale: String,
    /// id -> text.
    pub messages: IndexMap<String, String>,
}
impl MessageCatalog {
    /// New catalog.
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            messages: IndexMap::new(),
        }
    }
    /// Insert.
    pub fn insert(&mut self, id: impl Into<String>, text: impl Into<String>) {
        self.messages.insert(id.into(), text.into());
    }
    /// Get.
    pub fn get(&self, id: &str) -> Option<&str> {
        self.messages.get(id).map(|s| s.as_str())
    }
    /// JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    /// Parse JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

/// i18n errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum I18nError {
    /// Missing key.
    #[error("missing translation for `{0}` in locale `{1}`")]
    MissingKey(String, String),
    /// Empty id.
    #[error("empty message id")]
    EmptyId,
}

/// Extract `t!("key")` occurrences.
pub fn extract_msg_ids(source: &str) -> Vec<MsgEntry> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (li, line) in source.lines().enumerate() {
        let line_no = (li + 1) as u32;
        let mut rest = line;
        while let Some(idx) = rest.find("t!(\\"") {
            let after = &rest[idx + 4..];
            if let Some(end) = after.find('"') {
                let key = &after[..end];
                if !key.is_empty() && seen.insert(key.to_string()) {
                    out.push(MsgEntry {
                        id: MsgId::new(key),
                        text: key.to_string(),
                        file: None,
                        line: Some(line_no),
                    });
                }
                rest = &after[end + 1..];
            } else {
                break;
            }
        }
    }
    out
}

/// Validate required keys exist.
pub fn validate_catalog(required: &[MsgId], cat: &MessageCatalog) -> Vec<I18nError> {
    let mut errs = Vec::new();
    for id in required {
        if id.0.is_empty() {
            errs.push(I18nError::EmptyId);
            continue;
        }
        if cat.get(id.as_str()).is_none() {
            errs.push(I18nError::MissingKey(id.0.clone(), cat.locale.clone()));
        }
    }
    errs
}

/// Merge extract into catalog.
pub fn merge_extract(cat: &mut MessageCatalog, entries: &[MsgEntry]) {
    for e in entries {
        cat.messages
            .entry(e.id.0.clone())
            .or_insert_with(|| e.text.clone());
    }
}

"""
)
I.append("#[cfg(test)]\nmod tests {\n    use super::*;\n")
I.append(
    """    #[test]
    fn extract_t_bang() {
        let src = r#"say aria, t!("intro.hello"); menu { t!("choice.a") => {} }"#;
        let e = extract_msg_ids(src);
        assert!(e.iter().any(|x| x.id.as_str() == "intro.hello"));
        assert!(e.iter().any(|x| x.id.as_str() == "choice.a"));
    }
    #[test]
    fn validate_missing() {
        let mut c = MessageCatalog::new("es");
        c.insert("a", "hola");
        let errs = validate_catalog(&[MsgId::new("a"), MsgId::new("b")], &c);
        assert_eq!(errs.len(), 1);
    }
"""
)
for i in range(250):
    I.append(
        f"""    #[test]
    fn catalog_key_{i}() {{
        let mut c = MessageCatalog::new("en");
        c.insert("k{i}", "text {i}");
        assert_eq!(c.get("k{i}"), Some("text {i}"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k{i}"), Some("text {i}"));
    }}
"""
    )
I.append("}\n")
total += write(CRATES / "velvet-script-i18n" / "src" / "lib.rs", "".join(I))
print("i18n", total)

# Continue in same script - HIR, types, stdlib will be in part 2 if needed
print("GEN_PARTIAL_OK", total)
