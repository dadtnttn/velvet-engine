//! Resolve diagnostics.

#![allow(missing_docs)]

use velvet_script_hir::HirSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct ResolveDiag {
    pub code: &'static str,
    pub severity: ResolveSeverity,
    pub message: String,
    pub span: HirSpan,
    pub module: String,
}

impl ResolveDiag {
    pub fn error(
        code: &'static str,
        message: impl Into<String>,
        span: HirSpan,
        module: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: ResolveSeverity::Error,
            message: message.into(),
            span,
            module: module.into(),
        }
    }
    pub fn warning(
        code: &'static str,
        message: impl Into<String>,
        span: HirSpan,
        module: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: ResolveSeverity::Warning,
            message: message.into(),
            span,
            module: module.into(),
        }
    }
    pub fn display(&self) -> String {
        format!(
            "{}:{}: [{}] {}",
            self.module,
            self.span.display(),
            self.code,
            self.message
        )
    }
    pub fn is_error(&self) -> bool {
        matches!(self.severity, ResolveSeverity::Error)
    }
}

pub const RESOLVE_CODES: &[&str] = &[
    "E0001_unbound",
    "E0002_duplicate",
    "E0003_import_cycle",
    "E0004_private",
    "E0005_not_a_type",
    "E0006_not_a_value",
    "E0007_ambiguous",
    "E0008_bad_path",
    "E0009_missing_mod",
    "E0010_shadow_prelude",
    "E0011_mut_required",
    "E0012_const_assign",
    "E0013_scene_unbound",
    "E0014_layer_unbound",
    "E0015_msg_unbound",
    "E0016_screen_unbound",
    "E0017_character_unbound",
    "E0018_trait_unbound",
    "E0019_impl_orphan",
    "E0020_use_star_empty",
];

pub fn code_known(code: &str) -> bool {
    RESOLVE_CODES.contains(&code)
}

pub fn diag_e0001_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0001_unbound", name, span, module)
}

pub fn diag_e0002_duplicate(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0002_duplicate", name, span, module)
}

pub fn diag_e0003_import_cycle(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0003_import_cycle", name, span, module)
}

pub fn diag_e0004_private(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0004_private", name, span, module)
}

pub fn diag_e0005_not_a_type(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0005_not_a_type", name, span, module)
}

pub fn diag_e0006_not_a_value(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0006_not_a_value", name, span, module)
}

pub fn diag_e0007_ambiguous(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0007_ambiguous", name, span, module)
}

pub fn diag_e0008_bad_path(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0008_bad_path", name, span, module)
}

pub fn diag_e0009_missing_mod(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0009_missing_mod", name, span, module)
}

pub fn diag_e0010_shadow_prelude(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0010_shadow_prelude", name, span, module)
}

pub fn diag_e0011_mut_required(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0011_mut_required", name, span, module)
}

pub fn diag_e0012_const_assign(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0012_const_assign", name, span, module)
}

pub fn diag_e0013_scene_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0013_scene_unbound", name, span, module)
}

pub fn diag_e0014_layer_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0014_layer_unbound", name, span, module)
}

pub fn diag_e0015_msg_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0015_msg_unbound", name, span, module)
}

pub fn diag_e0016_screen_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0016_screen_unbound", name, span, module)
}

pub fn diag_e0017_character_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0017_character_unbound", name, span, module)
}

pub fn diag_e0018_trait_unbound(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0018_trait_unbound", name, span, module)
}

pub fn diag_e0019_impl_orphan(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0019_impl_orphan", name, span, module)
}

pub fn diag_e0020_use_star_empty(name: &str, span: HirSpan, module: &str) -> ResolveDiag {
    ResolveDiag::error("E0020_use_star_empty", name, span, module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::HirSpan;
    #[test]
    fn diagnostic_catalog_matches_the_public_resolver_contract() {
        let expected = [
            "E0001_unbound",
            "E0002_duplicate",
            "E0003_import_cycle",
            "E0004_private",
            "E0005_not_a_type",
            "E0006_not_a_value",
            "E0007_ambiguous",
            "E0008_bad_path",
            "E0009_missing_mod",
            "E0010_shadow_prelude",
            "E0011_mut_required",
            "E0012_const_assign",
            "E0013_scene_unbound",
            "E0014_layer_unbound",
            "E0015_msg_unbound",
            "E0016_screen_unbound",
            "E0017_character_unbound",
            "E0018_trait_unbound",
            "E0019_impl_orphan",
            "E0020_use_star_empty",
        ];
        assert_eq!(RESOLVE_CODES, expected);
        let unique: std::collections::HashSet<_> = RESOLVE_CODES.iter().copied().collect();
        assert_eq!(unique.len(), expected.len());
        assert!(expected.iter().all(|code| code_known(code)));
        assert!(!code_known("E9999_unknown"));
    }
    #[test]
    fn display_has_code() {
        let d = diag_e0001_unbound("x", HirSpan::unknown(), "m");
        assert!(d.display().contains("E0001"));
        assert!(d.is_error());
    }
}
