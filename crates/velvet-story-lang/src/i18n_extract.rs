//! Localization extract for Velvet Story (stable ids).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::ast::{Stmt, TopItem};
use crate::lower::stable_msg_id;
use crate::parser::parse;
use velvet_script_i18n::{MessageCatalog, MsgEntry, MsgId};

/// Extracted unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedMessages {
    /// File.
    pub file: String,
    /// Entries.
    pub entries: Vec<MsgEntry>,
}

/// Extract all dialogue / choice strings.
pub fn extract(source: &str, file: &str) -> ExtractedMessages {
    let parsed = parse(source, file);
    let mut entries = Vec::new();
    let mut seen = IndexMap::<String, String>::new();

    for item in &parsed.file.items {
        let TopItem::Scene(sc) = item else {
            continue;
        };
        walk(&sc.name, &sc.body, file, &mut entries, &mut seen);
    }

    ExtractedMessages {
        file: file.into(),
        entries,
    }
}

fn walk(
    scene: &str,
    body: &[Stmt],
    file: &str,
    entries: &mut Vec<MsgEntry>,
    seen: &mut IndexMap<String, String>,
) {
    for st in body {
        match st {
            Stmt::Dialogue {
                speaker,
                msg_id,
                text,
                span,
            } => {
                let id = msg_id
                    .clone()
                    .unwrap_or_else(|| stable_msg_id(scene, speaker, text));
                if seen.insert(id.clone(), text.clone()).is_none() {
                    entries.push(MsgEntry {
                        id: MsgId::new(id),
                        text: text.clone(),
                        file: Some(file.into()),
                        line: Some(span.line),
                    });
                }
            }
            Stmt::Choice { options, .. } => {
                for o in options {
                    let id = o
                        .msg_id
                        .clone()
                        .unwrap_or_else(|| stable_msg_id(scene, "choice", &o.label));
                    if seen.insert(id.clone(), o.label.clone()).is_none() {
                        entries.push(MsgEntry {
                            id: MsgId::new(id),
                            text: o.label.clone(),
                            file: Some(file.into()),
                            line: Some(o.span.line),
                        });
                    }
                    walk(scene, &o.body, file, entries, seen);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                walk(scene, then_body, file, entries, seen);
                if let Some(e) = else_body {
                    walk(scene, e, file, entries, seen);
                }
            }
            _ => {}
        }
    }
}

/// Build catalog for a locale from extraction (default text = source language).
pub fn to_catalog(extracted: &ExtractedMessages, locale: &str) -> MessageCatalog {
    let mut cat = MessageCatalog::new(locale);
    for e in &extracted.entries {
        cat.insert(e.id.as_str(), &e.text);
    }
    cat
}

/// Detect missing keys in a target catalog.
pub fn missing_keys(source: &ExtractedMessages, target: &MessageCatalog) -> Vec<String> {
    source
        .entries
        .iter()
        .filter(|e| target.get(e.id.as_str()).is_none())
        .map(|e| e.id.as_str().to_string())
        .collect()
}

/// Detect obsolete keys in target not present in source.
pub fn obsolete_keys(source: &ExtractedMessages, target: &MessageCatalog) -> Vec<String> {
    let live: std::collections::HashSet<_> = source
        .entries
        .iter()
        .map(|e| e.id.as_str().to_string())
        .collect();
    target
        .messages
        .keys()
        .filter(|k| !live.contains(*k))
        .cloned()
        .collect()
}
