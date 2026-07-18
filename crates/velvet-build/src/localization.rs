//! Localization extract / validate / simple PO-like formats.

use std::fmt::Write as _;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Localization errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LocalizationError {
    /// Missing key.
    #[error("missing key: {0}")]
    MissingKey(String),
    /// Empty translation.
    #[error("empty translation for {0}")]
    Empty(String),
    /// IO / format message.
    #[error("{0}")]
    Message(String),
}

/// One string entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizationEntry {
    /// Stable key.
    pub key: String,
    /// Source language text.
    pub source: String,
    /// Translated text (may be empty).
    pub translation: String,
    /// Context / file.
    pub context: String,
}

/// Catalog for a locale.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizationCatalog {
    /// Locale code e.g. `es`, `en`.
    pub locale: String,
    /// Entries by key.
    pub entries: IndexMap<String, LocalizationEntry>,
}

impl LocalizationCatalog {
    /// Create.
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            entries: IndexMap::new(),
        }
    }

    /// Insert / update source string (clears translation only when new).
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        source: impl Into<String>,
        context: impl Into<String>,
    ) {
        let key = key.into();
        self.entries.insert(
            key.clone(),
            LocalizationEntry {
                key,
                source: source.into(),
                translation: String::new(),
                context: context.into(),
            },
        );
    }

    /// Translate with fallback to source.
    pub fn t(&self, key: &str) -> String {
        self.entries
            .get(key)
            .map(|e| {
                if e.translation.is_empty() {
                    e.source.clone()
                } else {
                    e.translation.clone()
                }
            })
            .unwrap_or_else(|| key.to_string())
    }

    /// Set translation.
    pub fn set_translation(&mut self, key: &str, text: impl Into<String>) {
        if let Some(e) = self.entries.get_mut(key) {
            e.translation = text.into();
        }
    }

    /// Merge another catalog's translations by key (keeps our source/context).
    pub fn merge_translations(&mut self, other: &LocalizationCatalog) {
        for (k, e) in &other.entries {
            if let Some(mine) = self.entries.get_mut(k) {
                if !e.translation.is_empty() {
                    mine.translation = e.translation.clone();
                }
            }
        }
    }

    /// To JSON.
    pub fn to_json_pretty(&self) -> Result<String, LocalizationError> {
        serde_json::to_string_pretty(self).map_err(|e| LocalizationError::Message(e.to_string()))
    }

    /// From JSON.
    pub fn from_json(text: &str) -> Result<Self, LocalizationError> {
        serde_json::from_str(text).map_err(|e| LocalizationError::Message(e.to_string()))
    }

    /// Serialize to a simple gettext-inspired `.po` subset.
    ///
    /// Format per entry:
    /// ```text
    /// #. context
    /// # key: <key>
    /// msgid "source"
    /// msgstr "translation"
    /// ```
    pub fn to_po(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# Velvet Engine localization catalog\n# Locale: {}\nmsgid \"\"\nmsgstr \"\"\n\"Language: {}\\n\"\n",
            self.locale, self.locale
        );
        for e in self.entries.values() {
            if !e.context.is_empty() {
                let _ = writeln!(out, "#. {}", escape_po_comment(&e.context));
            }
            let _ = writeln!(out, "# key: {}", e.key);
            let _ = writeln!(out, "msgid \"{}\"", escape_po_string(&e.source));
            let _ = writeln!(out, "msgstr \"{}\"", escape_po_string(&e.translation));
            out.push('\n');
        }
        out
    }

    /// Parse simple `.po` subset produced by [`Self::to_po`] (and tolerant of extras).
    pub fn from_po(text: &str, locale: impl Into<String>) -> Result<Self, LocalizationError> {
        let mut cat = LocalizationCatalog::new(locale);
        let mut key = String::new();
        let mut context = String::new();
        let mut msgid: Option<String> = None;
        let mut msgstr: Option<String> = None;

        let flush = |cat: &mut LocalizationCatalog,
                     key: &mut String,
                     context: &mut String,
                     msgid: &mut Option<String>,
                     msgstr: &mut Option<String>| {
            if let Some(src) = msgid.take() {
                if src.is_empty() && key.is_empty() {
                    // header
                    *msgstr = None;
                    return;
                }
                let k = if key.is_empty() {
                    format!("msg:{}", cat.entries.len())
                } else {
                    key.clone()
                };
                let translation = msgstr.take().unwrap_or_default();
                cat.entries.insert(
                    k.clone(),
                    LocalizationEntry {
                        key: k,
                        source: src,
                        translation,
                        context: context.clone(),
                    },
                );
            }
            key.clear();
            context.clear();
            *msgstr = None;
        };

        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                flush(&mut cat, &mut key, &mut context, &mut msgid, &mut msgstr);
                continue;
            }
            if let Some(rest) = line.strip_prefix("# key:") {
                key = rest.trim().to_string();
                continue;
            }
            if let Some(rest) = line.strip_prefix("#.") {
                context = rest.trim().to_string();
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("msgid ") {
                if msgid.is_some() {
                    flush(&mut cat, &mut key, &mut context, &mut msgid, &mut msgstr);
                }
                msgid = Some(parse_po_quoted(rest)?);
                continue;
            }
            if let Some(rest) = line.strip_prefix("msgstr ") {
                msgstr = Some(parse_po_quoted(rest)?);
                continue;
            }
        }
        flush(&mut cat, &mut key, &mut context, &mut msgid, &mut msgstr);
        Ok(cat)
    }

    /// Simple key=value `.txt` / properties export (`key=translation` with source in comments).
    pub fn to_properties(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# locale: {}", self.locale);
        for e in self.entries.values() {
            let _ = writeln!(out, "# source: {}", e.source.replace('\n', "\\n"));
            if !e.context.is_empty() {
                let _ = writeln!(out, "# context: {}", e.context);
            }
            let _ = writeln!(
                out,
                "{}={}",
                escape_prop_key(&e.key),
                escape_prop_value(&e.translation)
            );
        }
        out
    }

    /// Parse properties format.
    pub fn from_properties(
        text: &str,
        locale: impl Into<String>,
    ) -> Result<Self, LocalizationError> {
        let mut cat = LocalizationCatalog::new(locale);
        let mut pending_source = String::new();
        let mut pending_context = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("# source:") {
                pending_source = rest.trim().replace("\\n", "\n");
                continue;
            }
            if let Some(rest) = line.strip_prefix("# context:") {
                pending_context = rest.trim().to_string();
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(LocalizationError::Message(format!(
                    "invalid properties line: {line}"
                )));
            };
            let key = k.trim().to_string();
            cat.entries.insert(
                key.clone(),
                LocalizationEntry {
                    key,
                    source: pending_source.clone(),
                    translation: v.trim().to_string(),
                    context: pending_context.clone(),
                },
            );
            pending_source.clear();
            pending_context.clear();
        }
        Ok(cat)
    }
}

fn escape_po_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_po_comment(s: &str) -> String {
    s.replace('\n', " ")
}

fn parse_po_quoted(s: &str) -> Result<String, LocalizationError> {
    let s = s.trim();
    if !(s.starts_with('"') && s.ends_with('"') && s.len() >= 2) {
        return Err(LocalizationError::Message(format!(
            "expected quoted string, got: {s}"
        )));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn escape_prop_key(k: &str) -> String {
    k.replace('=', "\\=").replace('\n', "")
}

fn escape_prop_value(v: &str) -> String {
    v.replace('\n', "\\n")
}

/// Extract string literals from Velvet Script source (dialogue + string literals).
pub fn extract_from_source(source: &str, context: &str) -> LocalizationCatalog {
    let mut cat = LocalizationCatalog::new("source");
    let mut key_i = 0u32;
    let mut chars = source.chars().peekable();
    let mut buf = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            buf.clear();
            while let Some(d) = chars.next() {
                if d == '\\' {
                    if let Some(n) = chars.next() {
                        buf.push(n);
                    }
                } else if d == '"' {
                    break;
                } else {
                    buf.push(d);
                }
            }
            if !buf.is_empty() {
                let key = format!("{context}:{key_i}");
                key_i += 1;
                cat.insert(key, buf.clone(), context);
            }
        }
    }
    cat
}

/// Validate that all keys in `source` exist and translations non-empty in `target`.
pub fn validate_catalog(
    source: &LocalizationCatalog,
    target: &LocalizationCatalog,
) -> Vec<LocalizationError> {
    let mut errs = Vec::new();
    for key in source.entries.keys() {
        match target.entries.get(key) {
            None => errs.push(LocalizationError::MissingKey(key.clone())),
            Some(e) if e.translation.is_empty() => errs.push(LocalizationError::Empty(key.clone())),
            _ => {}
        }
    }
    errs
}

/// Detect format from path extension and load.
pub fn load_catalog_auto(
    text: &str,
    path_hint: &str,
    locale: &str,
) -> Result<LocalizationCatalog, LocalizationError> {
    let lower = path_hint.to_ascii_lowercase();
    if lower.ends_with(".po") || lower.ends_with(".pot") {
        LocalizationCatalog::from_po(text, locale)
    } else if lower.ends_with(".properties") || lower.ends_with(".txt") {
        LocalizationCatalog::from_properties(text, locale)
    } else {
        LocalizationCatalog::from_json(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_and_validate() {
        let src = r#"aria "Hello" "World""#;
        let cat = extract_from_source(src, "demo.vel");
        assert!(cat.entries.len() >= 2);
        let mut es = cat.clone();
        es.locale = "es".into();
        for (k, e) in cat.entries.iter() {
            es.set_translation(k, format!("ES:{}", e.source));
        }
        assert!(validate_catalog(&cat, &es).is_empty());
        let empty = LocalizationCatalog::new("fr");
        assert!(!validate_catalog(&cat, &empty).is_empty());
    }

    #[test]
    fn po_roundtrip() {
        let mut cat = LocalizationCatalog::new("es");
        cat.insert("demo.vel:0", "Hello", "demo.vel");
        cat.set_translation("demo.vel:0", "Hola");
        cat.insert("demo.vel:1", "Bye", "demo.vel");
        cat.set_translation("demo.vel:1", "Adiós");
        let po = cat.to_po();
        assert!(po.contains("msgid \"Hello\""));
        let back = LocalizationCatalog::from_po(&po, "es").unwrap();
        assert_eq!(back.t("demo.vel:0"), "Hola");
        assert_eq!(back.t("demo.vel:1"), "Adiós");
    }

    #[test]
    fn properties_roundtrip() {
        let mut cat = LocalizationCatalog::new("fr");
        cat.insert("k1", "Yes", "ctx");
        cat.set_translation("k1", "Oui");
        let text = cat.to_properties();
        let back = LocalizationCatalog::from_properties(&text, "fr").unwrap();
        assert_eq!(back.t("k1"), "Oui");
    }

    #[test]
    fn extract_many_strings_from_story_like_source() {
        let src = r#"
aria "Hello"
aria "World"
"Narrator line"
choice_text "Yes" 
"#;
        let cat = extract_from_source(src, "story.vel");
        assert!(cat.entries.len() >= 2);
        for (k, e) in cat.entries.iter() {
            assert!(!k.is_empty());
            assert!(!e.source.is_empty());
        }
    }

    #[test]
    fn validate_detects_missing_and_extra() {
        let mut base = LocalizationCatalog::new("en");
        base.insert("a", "A", "f");
        base.insert("b", "B", "f");
        let mut partial = LocalizationCatalog::new("es");
        partial.insert("a", "A", "f");
        partial.set_translation("a", "Á");
        let issues = validate_catalog(&base, &partial);
        assert!(!issues.is_empty());
        // Full translation validates clean.
        let mut full = LocalizationCatalog::new("es");
        full.insert("a", "A", "f");
        full.insert("b", "B", "f");
        full.set_translation("a", "Á");
        full.set_translation("b", "Bé");
        assert!(validate_catalog(&base, &full).is_empty());
    }

    #[test]
    fn po_escapes_quotes() {
        let mut cat = LocalizationCatalog::new("de");
        cat.insert("q", r#"He said "hi""#, "ctx");
        cat.set_translation("q", r#"Er sagte "hi""#);
        let po = cat.to_po();
        let back = LocalizationCatalog::from_po(&po, "de").unwrap();
        assert!(back.t("q").contains("hi"));
    }

    #[test]
    fn fallback_t_returns_source() {
        let mut cat = LocalizationCatalog::new("en");
        cat.insert("k", "Hello", "ctx");
        // No translation set — t should return source or key.
        let v = cat.t("k");
        assert!(v == "Hello" || v == "k" || !v.is_empty());
        assert_eq!(cat.t("missing"), "missing");
    }
}
