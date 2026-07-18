//! Source locations for Velvet Story files.

use serde::{Deserialize, Serialize};

/// Byte/line span in a `.vstory` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
    /// UTF-8 start offset.
    pub start: usize,
    /// UTF-8 end offset.
    pub end: usize,
}

impl Span {
    /// Unknown / synthetic.
    pub fn unknown() -> Self {
        Self::default()
    }

    /// Construct at line/column.
    pub fn at(line: u32, column: u32, start: usize, end: usize) -> Self {
        Self {
            line,
            column,
            start,
            end,
        }
    }

    /// Human display `line:column`.
    pub fn display(&self) -> String {
        format!("{}:{}", self.line.max(1), self.column.max(1))
    }
}

/// File + span for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLoc {
    /// Path or virtual name.
    pub file: String,
    /// Span.
    pub span: Span,
}

impl SourceLoc {
    /// New.
    pub fn new(file: impl Into<String>, span: Span) -> Self {
        Self {
            file: file.into(),
            span,
        }
    }

    /// Display `file:line:column`.
    pub fn display(&self) -> String {
        format!("{}:{}", self.file, self.span.display())
    }
}
