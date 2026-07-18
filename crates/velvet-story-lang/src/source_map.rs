//! Source maps from generated VS2 / HIR back to Velvet Story.

use crate::ast::{Stmt, StoryFile, TopItem};
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
    /// Root / primary story file path (includes may add other files in entries).
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

    /// Push mapping attributed to the root [`Self::file`].
    pub fn push(
        &mut self,
        span: Span,
        node_kind: impl Into<String>,
        generated: impl Into<String>,
        pc: Option<u32>,
    ) {
        self.push_in_file(self.file.clone(), span, node_kind, generated, pc);
    }

    /// Push mapping with an explicit origin file (for `include`d scenes).
    pub fn push_in_file(
        &mut self,
        file: impl Into<String>,
        span: Span,
        node_kind: impl Into<String>,
        generated: impl Into<String>,
        pc: Option<u32>,
    ) {
        self.entries.push(MapEntry {
            origin: SourceLoc::new(file, span),
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

    /// Find first entry whose origin file path contains `needle`.
    pub fn by_file_substring(&self, needle: &str) -> Option<&MapEntry> {
        self.entries
            .iter()
            .find(|e| e.origin.file.contains(needle))
    }
}

/// Build a source map from a resolved [`StoryFile`], using each scene's
/// `origin_file` so included content points at the child path, not only the root.
pub fn map_from_story_file(file: &StoryFile) -> SourceMap {
    let mut map = SourceMap::new(&file.file);
    for item in &file.items {
        let TopItem::Scene(sc) = item else {
            continue;
        };
        let origin = sc
            .origin_file
            .clone()
            .unwrap_or_else(|| file.file.clone());
        map.push_in_file(
            origin.clone(),
            sc.span,
            "scene",
            format!("scene {}", sc.name),
            None,
        );
        walk_stmts(&mut map, &origin, &sc.name, &sc.body);
    }
    map
}

fn walk_stmts(map: &mut SourceMap, origin: &str, scene: &str, stmts: &[Stmt]) {
    for st in stmts {
        let (kind, gen, span) = match st {
            Stmt::Background { span, id } => ("background", id.clone(), *span),
            Stmt::Music { span, id } => ("music", id.clone(), *span),
            Stmt::Sound { span, id } => ("sound", id.clone(), *span),
            Stmt::Show { span, character, .. } => ("show", character.clone(), *span),
            Stmt::Hide { span, character } => ("hide", character.clone(), *span),
            Stmt::Dialogue { span, speaker, .. } => ("dialogue", speaker.clone(), *span),
            Stmt::Goto { span, target } => ("goto", target.clone(), *span),
            Stmt::CallScene { span, target } => ("call_scene", target.clone(), *span),
            Stmt::Return { span } => ("return", "return".into(), *span),
            Stmt::End { span } => ("end", "end".into(), *span),
            Stmt::Label { span, name } => ("label", name.clone(), *span),
            Stmt::Set { span, name, .. } => ("set", name.clone(), *span),
            Stmt::Add { span, name, .. } => ("add", name.clone(), *span),
            Stmt::Sub { span, name, .. } => ("sub", name.clone(), *span),
            Stmt::If { span, .. } => ("if", "cond".into(), *span),
            Stmt::Choice { span, .. } => ("choice", "menu".into(), *span),
            Stmt::CallCommand { span, name, .. } => ("call", name.clone(), *span),
            Stmt::Pause { span, .. } => ("pause", "await".into(), *span),
            Stmt::Transition { span, name } => ("transition", name.clone(), *span),
            Stmt::Comment { span, .. } => ("comment", format!("{scene}:comment"), *span),
        };
        map.push_in_file(origin, span, kind, gen, None);
        match st {
            Stmt::Choice { options, .. } => {
                for o in options {
                    walk_stmts(map, origin, scene, &o.body);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                walk_stmts(map, origin, scene, then_body);
                if let Some(e) = else_body {
                    walk_stmts(map, origin, scene, e);
                }
            }
            _ => {}
        }
    }
}
