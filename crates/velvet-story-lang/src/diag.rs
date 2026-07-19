//! Writer-friendly diagnostics for Velvet Story.

use crate::locale::{
    diag_locale, diag_message_for, diag_suggestion_for, suggestion_label_for, DiagLocale,
};
use crate::span::{SourceLoc, Span};
use serde::{Deserialize, Serialize};

/// Severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Error — blocks build/run.
    Error,
    /// Warning.
    Warning,
    /// Note / hint.
    Note,
}

/// One diagnostic aimed at writers (not compiler internals).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryDiag {
    /// Stable code, e.g. `VST001`.
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Natural-language message (localized at emission time).
    pub message: String,
    /// Optional suggestion block (localized at emission time).
    pub suggestion: Option<String>,
    /// Location in original `.vstory`.
    pub loc: SourceLoc,
    /// Related narrative node kind (scene, choice, …).
    pub node_kind: Option<String>,
    /// Locale used when this diagnostic was emitted (for display labels).
    #[serde(default)]
    pub locale: DiagLocale,
}

impl StoryDiag {
    /// Error from catalog key `code` with placeholder args (effective locale).
    pub fn error_key(
        code: impl Into<String>,
        args: &[(&str, &str)],
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        Self::error_key_locale(diag_locale(), code, args, file, span)
    }

    /// Error from catalog with an explicit locale (Studio multi-doc safe).
    pub fn error_key_locale(
        locale: DiagLocale,
        code: impl Into<String>,
        args: &[(&str, &str)],
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        let code = code.into();
        let message = diag_message_for(locale, &code, args);
        let suggestion = diag_suggestion_for(locale, &code, args);
        Self {
            code,
            severity: Severity::Error,
            message,
            suggestion,
            loc: SourceLoc::new(file, span),
            node_kind: None,
            locale,
        }
    }

    /// Warning from catalog (effective locale).
    pub fn warning_key(
        code: impl Into<String>,
        args: &[(&str, &str)],
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        Self::warning_key_locale(diag_locale(), code, args, file, span)
    }

    /// Warning from catalog with an explicit locale.
    pub fn warning_key_locale(
        locale: DiagLocale,
        code: impl Into<String>,
        args: &[(&str, &str)],
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        let code = code.into();
        let message = diag_message_for(locale, &code, args);
        let suggestion = diag_suggestion_for(locale, &code, args);
        Self {
            code,
            severity: Severity::Warning,
            message,
            suggestion,
            loc: SourceLoc::new(file, span),
            node_kind: None,
            locale,
        }
    }

    /// Error helper with an already-built message (prefer [`Self::error_key`]).
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            suggestion: None,
            loc: SourceLoc::new(file, span),
            node_kind: None,
            locale: diag_locale(),
        }
    }

    /// Warning helper with an already-built message (prefer [`Self::warning_key`]).
    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            suggestion: None,
            loc: SourceLoc::new(file, span),
            node_kind: None,
            locale: diag_locale(),
        }
    }

    /// Attach / replace suggestion.
    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }

    /// Attach node kind.
    pub fn with_node(mut self, kind: impl Into<String>) -> Self {
        self.node_kind = Some(kind.into());
        self
    }

    /// Full writer-facing display (suggestion label matches emission locale).
    pub fn display(&self) -> String {
        let mut out = format!("{}: [{}] {}", self.loc.display(), self.code, self.message);
        if let Some(s) = &self.suggestion {
            out.push_str("\n\n");
            out.push_str(suggestion_label_for(self.locale));
            out.push('\n');
            out.push_str(s);
        }
        out
    }

    /// Is error.
    pub fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }
}

/// Map an internal VS2-ish error into a writer message when possible.
pub fn adapt_internal(file: &str, span: Span, internal: &str) -> StoryDiag {
    let lower = internal.to_ascii_lowercase();
    if lower.contains("type") || lower.contains("bool") || lower.contains("condition") {
        return StoryDiag::error_key("VST050", &[], file, span).with_node("if");
    }
    if lower.contains("unresolved") || lower.contains("unbound") || lower.contains("unknown") {
        return StoryDiag::error_key("VST051", &[("detail", internal)], file, span);
    }
    StoryDiag::error_key("VST099", &[("detail", internal)], file, span)
}

/// Re-export locale controls for callers.
pub use crate::locale::{
    apply_locale_from_env, default_diag_locale, push_diag_locale, set_diag_locale,
    with_diag_locale, DiagLocaleGuard,
};
