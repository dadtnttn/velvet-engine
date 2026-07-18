//! Source maps from generated VS2 / HIR back to Velvet Story.

use crate::span::{SourceLoc, Span};
use serde::{Deserialize, Serialize};

/// One mapping entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEntry {
    /// Original story location.
    pub origin: SourceLoc,
    /// Narrative node kind.
    pub node_kind: String,
    /// Generated label / scene / note.
    pub generated: String,
    /// PC or index in lowered unit (if any).
    pub pc: Option<u32>,
}

/// Source map for a compilation unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMap {
    /// Story file path.
    pub file: String,
    /// Entries.
    pub entries: Vec<MapEntry>,
}

impl SourceMap {
    /// New.
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            entries: Vec::new(),
        }
    }

    /// Push mapping.
    pub fn push(
        &mut self,
        span: Span,
        node_kind: impl Into<String>,
        generated: impl Into<String>,
        pc: Option<u32>,
    ) {
        self.entries.push(MapEntry {
            origin: SourceLoc::new(self.file.clone(), span),
            node_kind: node_kind.into(),
            generated: generated.into(),
            pc,
        });
    }

    /// Find nearest entry by PC.
    pub fn by_pc(&self, pc: u32) -> Option<&MapEntry> {
        self.entries
            .iter()
            .filter(|e| e.pc.map(|p| p <= pc).unwrap_or(false))
            .max_by_key(|e| e.pc.unwrap_or(0))
    }

    /// Find by line.
    pub fn by_line(&self, line: u32) -> Option<&MapEntry> {
        self.entries
            .iter()
            .find(|e| e.origin.span.line == line)
    }
}
