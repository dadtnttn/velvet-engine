//! Prelude names visible without import in VS2 edition 2.

#![allow(missing_docs)]

#[derive(Debug, Clone, Copy)]
pub struct PreludeEntry {
    pub name: &'static str,
    pub kind: &'static str,
    pub ty_hint: &'static str,
}

pub static PRELUDE: &[PreludeEntry] = &[
    PreludeEntry {
        name: "print",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "abs",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "min",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "max",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "floor",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "ceil",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "clamp",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "len",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "concat",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "str",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "Some",
        kind: "variant",
        ty_hint: "variant",
    },
    PreludeEntry {
        name: "None",
        kind: "variant",
        ty_hint: "variant",
    },
    PreludeEntry {
        name: "Ok",
        kind: "variant",
        ty_hint: "variant",
    },
    PreludeEntry {
        name: "Err",
        kind: "variant",
        ty_hint: "variant",
    },
    PreludeEntry {
        name: "Option",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "Result",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "Vec",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "String",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "LayerId",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "SceneId",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "MsgId",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "ScriptError",
        kind: "type",
        ty_hint: "type",
    },
    PreludeEntry {
        name: "push_layer",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "pop_layer",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "show_layer",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "hide_layer",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "t",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "say",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "jump",
        kind: "fn",
        ty_hint: "fn",
    },
    PreludeEntry {
        name: "call_scene",
        kind: "fn",
        ty_hint: "fn",
    },
];

pub fn is_prelude(name: &str) -> bool {
    PRELUDE.iter().any(|e| e.name == name)
}
pub fn prelude_ty(name: &str) -> Option<&'static str> {
    PRELUDE.iter().find(|e| e.name == name).map(|e| e.ty_hint)
}
pub fn prelude_kind(name: &str) -> Option<&'static str> {
    PRELUDE.iter().find(|e| e.name == name).map(|e| e.kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn core_prelude() {
        assert!(is_prelude("print"));
        assert!(is_prelude("LayerId"));
        assert!(is_prelude("MsgId"));
        assert!(!is_prelude("not_a_real_name_xyz"));
    }
}
