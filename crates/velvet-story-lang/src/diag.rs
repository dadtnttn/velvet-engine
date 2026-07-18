//! Writer-friendly diagnostics for Velvet Story.

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
    /// Natural-language message.
    pub message: String,
    /// Optional suggestion block.
    pub suggestion: Option<String>,
    /// Location in original `.vstory`.
    pub loc: SourceLoc,
    /// Related narrative node kind (scene, choice, …).
    pub node_kind: Option<String>,
}

impl StoryDiag {
    /// Error helper.
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
        }
    }

    /// Warning helper.
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
        }
    }

    /// Attach suggestion.
    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestion = Some(s.into());
        self
    }

    /// Attach node kind.
    pub fn with_node(mut self, kind: impl Into<String>) -> Self {
        self.node_kind = Some(kind.into());
        self
    }

    /// Full writer-facing display.
    pub fn display(&self) -> String {
        let mut out = format!("{}: [{}] {}", self.loc.display(), self.code, self.message);
        if let Some(s) = &self.suggestion {
            out.push_str("\n\nSugerencia:\n");
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
        return StoryDiag::error(
            "VST050",
            "La condición de \"if\" debe ser verdadero o falso (un número, una variable, o una comparación).",
            file,
            span,
        )
        .with_suggestion("if affection >= 3:")
        .with_node("if");
    }
    if lower.contains("unresolved") || lower.contains("unbound") || lower.contains("unknown") {
        return StoryDiag::error(
            "VST051",
            format!("No se pudo resolver un nombre interno. Detalle técnico: {internal}"),
            file,
            span,
        );
    }
    StoryDiag::error(
        "VST099",
        format!("Error al preparar la historia. Detalle: {internal}"),
        file,
        span,
    )
}
