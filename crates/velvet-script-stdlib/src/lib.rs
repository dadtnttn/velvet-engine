//! Typed stdlib descriptors for Velvet Script 2 (prelude signatures).
//!
//! Real function names only — no `abs_1`..`abs_N` padding clones.

#![deny(missing_docs)]

use velvet_script_hir::{HirTy, PrimTy};

/// Stdlib function signature.
#[derive(Debug, Clone)]
pub struct StdFn {
    /// Name.
    pub name: &'static str,
    /// Module path.
    pub module: &'static str,
    /// Params.
    pub params: &'static [&'static str],
    /// Return type name.
    pub ret: &'static str,
    /// Docs.
    pub doc: &'static str,
}

/// All real stdlib functions (unique names).
pub static STDLIB: &[StdFn] = &[
    // math
    StdFn {
        name: "abs",
        module: "math",
        params: &["x"],
        ret: "f64",
        doc: "Absolute value",
    },
    StdFn {
        name: "min",
        module: "math",
        params: &["a", "b"],
        ret: "f64",
        doc: "Minimum of two values",
    },
    StdFn {
        name: "max",
        module: "math",
        params: &["a", "b"],
        ret: "f64",
        doc: "Maximum of two values",
    },
    StdFn {
        name: "clamp",
        module: "math",
        params: &["x", "lo", "hi"],
        ret: "f64",
        doc: "Clamp x into [lo, hi]",
    },
    StdFn {
        name: "floor",
        module: "math",
        params: &["x"],
        ret: "f64",
        doc: "Floor",
    },
    StdFn {
        name: "ceil",
        module: "math",
        params: &["x"],
        ret: "f64",
        doc: "Ceil",
    },
    StdFn {
        name: "sqrt",
        module: "math",
        params: &["x"],
        ret: "f64",
        doc: "Square root",
    },
    // string
    StdFn {
        name: "len",
        module: "string",
        params: &["s"],
        ret: "i32",
        doc: "String / collection length",
    },
    StdFn {
        name: "contains",
        module: "string",
        params: &["s", "sub"],
        ret: "bool",
        doc: "Substring check",
    },
    StdFn {
        name: "concat",
        module: "string",
        params: &["a", "b"],
        ret: "str",
        doc: "Concatenate",
    },
    // util
    StdFn {
        name: "print",
        module: "util",
        params: &["x"],
        ret: "()",
        doc: "Print to host output",
    },
    StdFn {
        name: "str",
        module: "util",
        params: &["x"],
        ret: "str",
        doc: "Convert to string",
    },
    // story / layers / i18n (host-facing)
    StdFn {
        name: "push_layer",
        module: "layer",
        params: &["id"],
        ret: "()",
        doc: "Push UI layer",
    },
    StdFn {
        name: "pop_layer",
        module: "layer",
        params: &[],
        ret: "()",
        doc: "Pop UI layer",
    },
    StdFn {
        name: "show_layer",
        module: "layer",
        params: &["id"],
        ret: "()",
        doc: "Show layer",
    },
    StdFn {
        name: "hide_layer",
        module: "layer",
        params: &["id"],
        ret: "()",
        doc: "Hide layer",
    },
    StdFn {
        name: "say",
        module: "story",
        params: &["speaker", "msg"],
        ret: "()",
        doc: "Dialogue line",
    },
    StdFn {
        name: "jump",
        module: "story",
        params: &["scene"],
        ret: "()",
        doc: "Jump to scene",
    },
    StdFn {
        name: "t",
        module: "i18n",
        params: &["key"],
        ret: "str",
        doc: "Translate message key",
    },
];

/// Lookup stdlib function by exact name.
pub fn find_std(name: &str) -> Option<&'static StdFn> {
    STDLIB.iter().find(|f| f.name == name)
}

/// Names in a module.
pub fn module_fns(module: &str) -> Vec<&'static StdFn> {
    STDLIB.iter().filter(|f| f.module == module).collect()
}

/// Map ret name to HirTy roughly.
pub fn ret_ty(name: &str) -> HirTy {
    match name {
        "i32" => HirTy::Prim(PrimTy::I32),
        "f64" => HirTy::Prim(PrimTy::F64),
        "bool" => HirTy::Prim(PrimTy::Bool),
        "str" => HirTy::Prim(PrimTy::Str),
        "LayerId" => HirTy::LayerId,
        "MsgId" => HirTy::MsgId,
        "()" => HirTy::Prim(PrimTy::Unit),
        _ => HirTy::Prim(PrimTy::Unit),
    }
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdlib_unique_real_names() {
        assert!(STDLIB.len() >= 10);
        assert!(STDLIB.len() < 80, "stdlib must not be padded with clones");
        let mut names = std::collections::HashSet::new();
        for f in STDLIB {
            assert!(
                !f.name.contains('_') || !f.name.chars().last().unwrap().is_ascii_digit(),
                "padded name rejected: {}",
                f.name
            );
            assert!(names.insert(f.name), "duplicate std name {}", f.name);
        }
        assert!(find_std("abs").is_some());
        assert!(find_std("print").is_some());
        assert!(find_std("push_layer").is_some());
        assert!(find_std("abs_1").is_none());
        assert!(find_std("play_bgm_4").is_none());
    }

    #[test]
    fn ret_ty_known() {
        assert_eq!(ret_ty("i32"), HirTy::Prim(PrimTy::I32));
        assert_eq!(ret_ty("bool"), HirTy::Prim(PrimTy::Bool));
        assert_eq!(ret_ty("()"), HirTy::Prim(PrimTy::Unit));
    }

    #[test]
    fn modules_partition() {
        assert!(!module_fns("math").is_empty());
        assert!(!module_fns("story").is_empty());
        assert!(module_fns("nope").is_empty());
    }
}
