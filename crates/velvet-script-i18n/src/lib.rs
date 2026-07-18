//! Message catalogs and extract/validate for Velvet Script 2.

#![deny(missing_docs)]

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Message key.
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
        while let Some(idx) = rest.find("t!(\"") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_json_roundtrip_exact() {
        let mut cat = MessageCatalog::new("es");
        cat.insert("hello", "Hola");
        let json = cat.to_json().unwrap();
        let back = MessageCatalog::from_json(&json).unwrap();
        assert_eq!(back.get("hello"), Some("Hola"));
        assert_eq!(back.locale, "es");
    }

    #[test]
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
}
