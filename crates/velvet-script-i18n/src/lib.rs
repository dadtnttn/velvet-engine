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
    #[test]
    fn catalog_key_0() {
        let mut c = MessageCatalog::new("en");
        c.insert("k0", "text 0");
        assert_eq!(c.get("k0"), Some("text 0"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k0"), Some("text 0"));
    }
    #[test]
    fn catalog_key_1() {
        let mut c = MessageCatalog::new("en");
        c.insert("k1", "text 1");
        assert_eq!(c.get("k1"), Some("text 1"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k1"), Some("text 1"));
    }
    #[test]
    fn catalog_key_2() {
        let mut c = MessageCatalog::new("en");
        c.insert("k2", "text 2");
        assert_eq!(c.get("k2"), Some("text 2"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k2"), Some("text 2"));
    }
    #[test]
    fn catalog_key_3() {
        let mut c = MessageCatalog::new("en");
        c.insert("k3", "text 3");
        assert_eq!(c.get("k3"), Some("text 3"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k3"), Some("text 3"));
    }
    #[test]
    fn catalog_key_4() {
        let mut c = MessageCatalog::new("en");
        c.insert("k4", "text 4");
        assert_eq!(c.get("k4"), Some("text 4"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k4"), Some("text 4"));
    }
    #[test]
    fn catalog_key_5() {
        let mut c = MessageCatalog::new("en");
        c.insert("k5", "text 5");
        assert_eq!(c.get("k5"), Some("text 5"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k5"), Some("text 5"));
    }
    #[test]
    fn catalog_key_6() {
        let mut c = MessageCatalog::new("en");
        c.insert("k6", "text 6");
        assert_eq!(c.get("k6"), Some("text 6"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k6"), Some("text 6"));
    }
    #[test]
    fn catalog_key_7() {
        let mut c = MessageCatalog::new("en");
        c.insert("k7", "text 7");
        assert_eq!(c.get("k7"), Some("text 7"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k7"), Some("text 7"));
    }
    #[test]
    fn catalog_key_8() {
        let mut c = MessageCatalog::new("en");
        c.insert("k8", "text 8");
        assert_eq!(c.get("k8"), Some("text 8"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k8"), Some("text 8"));
    }
    #[test]
    fn catalog_key_9() {
        let mut c = MessageCatalog::new("en");
        c.insert("k9", "text 9");
        assert_eq!(c.get("k9"), Some("text 9"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k9"), Some("text 9"));
    }
    #[test]
    fn catalog_key_10() {
        let mut c = MessageCatalog::new("en");
        c.insert("k10", "text 10");
        assert_eq!(c.get("k10"), Some("text 10"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k10"), Some("text 10"));
    }
    #[test]
    fn catalog_key_11() {
        let mut c = MessageCatalog::new("en");
        c.insert("k11", "text 11");
        assert_eq!(c.get("k11"), Some("text 11"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k11"), Some("text 11"));
    }
    #[test]
    fn catalog_key_12() {
        let mut c = MessageCatalog::new("en");
        c.insert("k12", "text 12");
        assert_eq!(c.get("k12"), Some("text 12"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k12"), Some("text 12"));
    }
    #[test]
    fn catalog_key_13() {
        let mut c = MessageCatalog::new("en");
        c.insert("k13", "text 13");
        assert_eq!(c.get("k13"), Some("text 13"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k13"), Some("text 13"));
    }
    #[test]
    fn catalog_key_14() {
        let mut c = MessageCatalog::new("en");
        c.insert("k14", "text 14");
        assert_eq!(c.get("k14"), Some("text 14"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k14"), Some("text 14"));
    }
    #[test]
    fn catalog_key_15() {
        let mut c = MessageCatalog::new("en");
        c.insert("k15", "text 15");
        assert_eq!(c.get("k15"), Some("text 15"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k15"), Some("text 15"));
    }
    #[test]
    fn catalog_key_16() {
        let mut c = MessageCatalog::new("en");
        c.insert("k16", "text 16");
        assert_eq!(c.get("k16"), Some("text 16"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k16"), Some("text 16"));
    }
    #[test]
    fn catalog_key_17() {
        let mut c = MessageCatalog::new("en");
        c.insert("k17", "text 17");
        assert_eq!(c.get("k17"), Some("text 17"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k17"), Some("text 17"));
    }
    #[test]
    fn catalog_key_18() {
        let mut c = MessageCatalog::new("en");
        c.insert("k18", "text 18");
        assert_eq!(c.get("k18"), Some("text 18"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k18"), Some("text 18"));
    }
    #[test]
    fn catalog_key_19() {
        let mut c = MessageCatalog::new("en");
        c.insert("k19", "text 19");
        assert_eq!(c.get("k19"), Some("text 19"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k19"), Some("text 19"));
    }
    #[test]
    fn catalog_key_20() {
        let mut c = MessageCatalog::new("en");
        c.insert("k20", "text 20");
        assert_eq!(c.get("k20"), Some("text 20"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k20"), Some("text 20"));
    }
    #[test]
    fn catalog_key_21() {
        let mut c = MessageCatalog::new("en");
        c.insert("k21", "text 21");
        assert_eq!(c.get("k21"), Some("text 21"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k21"), Some("text 21"));
    }
    #[test]
    fn catalog_key_22() {
        let mut c = MessageCatalog::new("en");
        c.insert("k22", "text 22");
        assert_eq!(c.get("k22"), Some("text 22"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k22"), Some("text 22"));
    }
    #[test]
    fn catalog_key_23() {
        let mut c = MessageCatalog::new("en");
        c.insert("k23", "text 23");
        assert_eq!(c.get("k23"), Some("text 23"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k23"), Some("text 23"));
    }
    #[test]
    fn catalog_key_24() {
        let mut c = MessageCatalog::new("en");
        c.insert("k24", "text 24");
        assert_eq!(c.get("k24"), Some("text 24"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k24"), Some("text 24"));
    }
    #[test]
    fn catalog_key_25() {
        let mut c = MessageCatalog::new("en");
        c.insert("k25", "text 25");
        assert_eq!(c.get("k25"), Some("text 25"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k25"), Some("text 25"));
    }
    #[test]
    fn catalog_key_26() {
        let mut c = MessageCatalog::new("en");
        c.insert("k26", "text 26");
        assert_eq!(c.get("k26"), Some("text 26"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k26"), Some("text 26"));
    }
    #[test]
    fn catalog_key_27() {
        let mut c = MessageCatalog::new("en");
        c.insert("k27", "text 27");
        assert_eq!(c.get("k27"), Some("text 27"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k27"), Some("text 27"));
    }
    #[test]
    fn catalog_key_28() {
        let mut c = MessageCatalog::new("en");
        c.insert("k28", "text 28");
        assert_eq!(c.get("k28"), Some("text 28"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k28"), Some("text 28"));
    }
    #[test]
    fn catalog_key_29() {
        let mut c = MessageCatalog::new("en");
        c.insert("k29", "text 29");
        assert_eq!(c.get("k29"), Some("text 29"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k29"), Some("text 29"));
    }
    #[test]
    fn catalog_key_30() {
        let mut c = MessageCatalog::new("en");
        c.insert("k30", "text 30");
        assert_eq!(c.get("k30"), Some("text 30"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k30"), Some("text 30"));
    }
    #[test]
    fn catalog_key_31() {
        let mut c = MessageCatalog::new("en");
        c.insert("k31", "text 31");
        assert_eq!(c.get("k31"), Some("text 31"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k31"), Some("text 31"));
    }
    #[test]
    fn catalog_key_32() {
        let mut c = MessageCatalog::new("en");
        c.insert("k32", "text 32");
        assert_eq!(c.get("k32"), Some("text 32"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k32"), Some("text 32"));
    }
    #[test]
    fn catalog_key_33() {
        let mut c = MessageCatalog::new("en");
        c.insert("k33", "text 33");
        assert_eq!(c.get("k33"), Some("text 33"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k33"), Some("text 33"));
    }
    #[test]
    fn catalog_key_34() {
        let mut c = MessageCatalog::new("en");
        c.insert("k34", "text 34");
        assert_eq!(c.get("k34"), Some("text 34"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k34"), Some("text 34"));
    }
    #[test]
    fn catalog_key_35() {
        let mut c = MessageCatalog::new("en");
        c.insert("k35", "text 35");
        assert_eq!(c.get("k35"), Some("text 35"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k35"), Some("text 35"));
    }
    #[test]
    fn catalog_key_36() {
        let mut c = MessageCatalog::new("en");
        c.insert("k36", "text 36");
        assert_eq!(c.get("k36"), Some("text 36"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k36"), Some("text 36"));
    }
    #[test]
    fn catalog_key_37() {
        let mut c = MessageCatalog::new("en");
        c.insert("k37", "text 37");
        assert_eq!(c.get("k37"), Some("text 37"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k37"), Some("text 37"));
    }
    #[test]
    fn catalog_key_38() {
        let mut c = MessageCatalog::new("en");
        c.insert("k38", "text 38");
        assert_eq!(c.get("k38"), Some("text 38"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k38"), Some("text 38"));
    }
    #[test]
    fn catalog_key_39() {
        let mut c = MessageCatalog::new("en");
        c.insert("k39", "text 39");
        assert_eq!(c.get("k39"), Some("text 39"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k39"), Some("text 39"));
    }
    #[test]
    fn catalog_key_40() {
        let mut c = MessageCatalog::new("en");
        c.insert("k40", "text 40");
        assert_eq!(c.get("k40"), Some("text 40"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k40"), Some("text 40"));
    }
    #[test]
    fn catalog_key_41() {
        let mut c = MessageCatalog::new("en");
        c.insert("k41", "text 41");
        assert_eq!(c.get("k41"), Some("text 41"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k41"), Some("text 41"));
    }
    #[test]
    fn catalog_key_42() {
        let mut c = MessageCatalog::new("en");
        c.insert("k42", "text 42");
        assert_eq!(c.get("k42"), Some("text 42"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k42"), Some("text 42"));
    }
    #[test]
    fn catalog_key_43() {
        let mut c = MessageCatalog::new("en");
        c.insert("k43", "text 43");
        assert_eq!(c.get("k43"), Some("text 43"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k43"), Some("text 43"));
    }
    #[test]
    fn catalog_key_44() {
        let mut c = MessageCatalog::new("en");
        c.insert("k44", "text 44");
        assert_eq!(c.get("k44"), Some("text 44"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k44"), Some("text 44"));
    }
    #[test]
    fn catalog_key_45() {
        let mut c = MessageCatalog::new("en");
        c.insert("k45", "text 45");
        assert_eq!(c.get("k45"), Some("text 45"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k45"), Some("text 45"));
    }
    #[test]
    fn catalog_key_46() {
        let mut c = MessageCatalog::new("en");
        c.insert("k46", "text 46");
        assert_eq!(c.get("k46"), Some("text 46"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k46"), Some("text 46"));
    }
    #[test]
    fn catalog_key_47() {
        let mut c = MessageCatalog::new("en");
        c.insert("k47", "text 47");
        assert_eq!(c.get("k47"), Some("text 47"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k47"), Some("text 47"));
    }
    #[test]
    fn catalog_key_48() {
        let mut c = MessageCatalog::new("en");
        c.insert("k48", "text 48");
        assert_eq!(c.get("k48"), Some("text 48"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k48"), Some("text 48"));
    }
    #[test]
    fn catalog_key_49() {
        let mut c = MessageCatalog::new("en");
        c.insert("k49", "text 49");
        assert_eq!(c.get("k49"), Some("text 49"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k49"), Some("text 49"));
    }
    #[test]
    fn catalog_key_50() {
        let mut c = MessageCatalog::new("en");
        c.insert("k50", "text 50");
        assert_eq!(c.get("k50"), Some("text 50"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k50"), Some("text 50"));
    }
    #[test]
    fn catalog_key_51() {
        let mut c = MessageCatalog::new("en");
        c.insert("k51", "text 51");
        assert_eq!(c.get("k51"), Some("text 51"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k51"), Some("text 51"));
    }
    #[test]
    fn catalog_key_52() {
        let mut c = MessageCatalog::new("en");
        c.insert("k52", "text 52");
        assert_eq!(c.get("k52"), Some("text 52"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k52"), Some("text 52"));
    }
    #[test]
    fn catalog_key_53() {
        let mut c = MessageCatalog::new("en");
        c.insert("k53", "text 53");
        assert_eq!(c.get("k53"), Some("text 53"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k53"), Some("text 53"));
    }
    #[test]
    fn catalog_key_54() {
        let mut c = MessageCatalog::new("en");
        c.insert("k54", "text 54");
        assert_eq!(c.get("k54"), Some("text 54"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k54"), Some("text 54"));
    }
    #[test]
    fn catalog_key_55() {
        let mut c = MessageCatalog::new("en");
        c.insert("k55", "text 55");
        assert_eq!(c.get("k55"), Some("text 55"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k55"), Some("text 55"));
    }
    #[test]
    fn catalog_key_56() {
        let mut c = MessageCatalog::new("en");
        c.insert("k56", "text 56");
        assert_eq!(c.get("k56"), Some("text 56"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k56"), Some("text 56"));
    }
    #[test]
    fn catalog_key_57() {
        let mut c = MessageCatalog::new("en");
        c.insert("k57", "text 57");
        assert_eq!(c.get("k57"), Some("text 57"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k57"), Some("text 57"));
    }
    #[test]
    fn catalog_key_58() {
        let mut c = MessageCatalog::new("en");
        c.insert("k58", "text 58");
        assert_eq!(c.get("k58"), Some("text 58"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k58"), Some("text 58"));
    }
    #[test]
    fn catalog_key_59() {
        let mut c = MessageCatalog::new("en");
        c.insert("k59", "text 59");
        assert_eq!(c.get("k59"), Some("text 59"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k59"), Some("text 59"));
    }
    #[test]
    fn catalog_key_60() {
        let mut c = MessageCatalog::new("en");
        c.insert("k60", "text 60");
        assert_eq!(c.get("k60"), Some("text 60"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k60"), Some("text 60"));
    }
    #[test]
    fn catalog_key_61() {
        let mut c = MessageCatalog::new("en");
        c.insert("k61", "text 61");
        assert_eq!(c.get("k61"), Some("text 61"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k61"), Some("text 61"));
    }
    #[test]
    fn catalog_key_62() {
        let mut c = MessageCatalog::new("en");
        c.insert("k62", "text 62");
        assert_eq!(c.get("k62"), Some("text 62"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k62"), Some("text 62"));
    }
    #[test]
    fn catalog_key_63() {
        let mut c = MessageCatalog::new("en");
        c.insert("k63", "text 63");
        assert_eq!(c.get("k63"), Some("text 63"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k63"), Some("text 63"));
    }
    #[test]
    fn catalog_key_64() {
        let mut c = MessageCatalog::new("en");
        c.insert("k64", "text 64");
        assert_eq!(c.get("k64"), Some("text 64"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k64"), Some("text 64"));
    }
    #[test]
    fn catalog_key_65() {
        let mut c = MessageCatalog::new("en");
        c.insert("k65", "text 65");
        assert_eq!(c.get("k65"), Some("text 65"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k65"), Some("text 65"));
    }
    #[test]
    fn catalog_key_66() {
        let mut c = MessageCatalog::new("en");
        c.insert("k66", "text 66");
        assert_eq!(c.get("k66"), Some("text 66"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k66"), Some("text 66"));
    }
    #[test]
    fn catalog_key_67() {
        let mut c = MessageCatalog::new("en");
        c.insert("k67", "text 67");
        assert_eq!(c.get("k67"), Some("text 67"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k67"), Some("text 67"));
    }
    #[test]
    fn catalog_key_68() {
        let mut c = MessageCatalog::new("en");
        c.insert("k68", "text 68");
        assert_eq!(c.get("k68"), Some("text 68"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k68"), Some("text 68"));
    }
    #[test]
    fn catalog_key_69() {
        let mut c = MessageCatalog::new("en");
        c.insert("k69", "text 69");
        assert_eq!(c.get("k69"), Some("text 69"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k69"), Some("text 69"));
    }
    #[test]
    fn catalog_key_70() {
        let mut c = MessageCatalog::new("en");
        c.insert("k70", "text 70");
        assert_eq!(c.get("k70"), Some("text 70"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k70"), Some("text 70"));
    }
    #[test]
    fn catalog_key_71() {
        let mut c = MessageCatalog::new("en");
        c.insert("k71", "text 71");
        assert_eq!(c.get("k71"), Some("text 71"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k71"), Some("text 71"));
    }
    #[test]
    fn catalog_key_72() {
        let mut c = MessageCatalog::new("en");
        c.insert("k72", "text 72");
        assert_eq!(c.get("k72"), Some("text 72"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k72"), Some("text 72"));
    }
    #[test]
    fn catalog_key_73() {
        let mut c = MessageCatalog::new("en");
        c.insert("k73", "text 73");
        assert_eq!(c.get("k73"), Some("text 73"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k73"), Some("text 73"));
    }
    #[test]
    fn catalog_key_74() {
        let mut c = MessageCatalog::new("en");
        c.insert("k74", "text 74");
        assert_eq!(c.get("k74"), Some("text 74"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k74"), Some("text 74"));
    }
    #[test]
    fn catalog_key_75() {
        let mut c = MessageCatalog::new("en");
        c.insert("k75", "text 75");
        assert_eq!(c.get("k75"), Some("text 75"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k75"), Some("text 75"));
    }
    #[test]
    fn catalog_key_76() {
        let mut c = MessageCatalog::new("en");
        c.insert("k76", "text 76");
        assert_eq!(c.get("k76"), Some("text 76"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k76"), Some("text 76"));
    }
    #[test]
    fn catalog_key_77() {
        let mut c = MessageCatalog::new("en");
        c.insert("k77", "text 77");
        assert_eq!(c.get("k77"), Some("text 77"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k77"), Some("text 77"));
    }
    #[test]
    fn catalog_key_78() {
        let mut c = MessageCatalog::new("en");
        c.insert("k78", "text 78");
        assert_eq!(c.get("k78"), Some("text 78"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k78"), Some("text 78"));
    }
    #[test]
    fn catalog_key_79() {
        let mut c = MessageCatalog::new("en");
        c.insert("k79", "text 79");
        assert_eq!(c.get("k79"), Some("text 79"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k79"), Some("text 79"));
    }
    #[test]
    fn catalog_key_80() {
        let mut c = MessageCatalog::new("en");
        c.insert("k80", "text 80");
        assert_eq!(c.get("k80"), Some("text 80"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k80"), Some("text 80"));
    }
    #[test]
    fn catalog_key_81() {
        let mut c = MessageCatalog::new("en");
        c.insert("k81", "text 81");
        assert_eq!(c.get("k81"), Some("text 81"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k81"), Some("text 81"));
    }
    #[test]
    fn catalog_key_82() {
        let mut c = MessageCatalog::new("en");
        c.insert("k82", "text 82");
        assert_eq!(c.get("k82"), Some("text 82"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k82"), Some("text 82"));
    }
    #[test]
    fn catalog_key_83() {
        let mut c = MessageCatalog::new("en");
        c.insert("k83", "text 83");
        assert_eq!(c.get("k83"), Some("text 83"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k83"), Some("text 83"));
    }
    #[test]
    fn catalog_key_84() {
        let mut c = MessageCatalog::new("en");
        c.insert("k84", "text 84");
        assert_eq!(c.get("k84"), Some("text 84"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k84"), Some("text 84"));
    }
    #[test]
    fn catalog_key_85() {
        let mut c = MessageCatalog::new("en");
        c.insert("k85", "text 85");
        assert_eq!(c.get("k85"), Some("text 85"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k85"), Some("text 85"));
    }
    #[test]
    fn catalog_key_86() {
        let mut c = MessageCatalog::new("en");
        c.insert("k86", "text 86");
        assert_eq!(c.get("k86"), Some("text 86"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k86"), Some("text 86"));
    }
    #[test]
    fn catalog_key_87() {
        let mut c = MessageCatalog::new("en");
        c.insert("k87", "text 87");
        assert_eq!(c.get("k87"), Some("text 87"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k87"), Some("text 87"));
    }
    #[test]
    fn catalog_key_88() {
        let mut c = MessageCatalog::new("en");
        c.insert("k88", "text 88");
        assert_eq!(c.get("k88"), Some("text 88"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k88"), Some("text 88"));
    }
    #[test]
    fn catalog_key_89() {
        let mut c = MessageCatalog::new("en");
        c.insert("k89", "text 89");
        assert_eq!(c.get("k89"), Some("text 89"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k89"), Some("text 89"));
    }
    #[test]
    fn catalog_key_90() {
        let mut c = MessageCatalog::new("en");
        c.insert("k90", "text 90");
        assert_eq!(c.get("k90"), Some("text 90"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k90"), Some("text 90"));
    }
    #[test]
    fn catalog_key_91() {
        let mut c = MessageCatalog::new("en");
        c.insert("k91", "text 91");
        assert_eq!(c.get("k91"), Some("text 91"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k91"), Some("text 91"));
    }
    #[test]
    fn catalog_key_92() {
        let mut c = MessageCatalog::new("en");
        c.insert("k92", "text 92");
        assert_eq!(c.get("k92"), Some("text 92"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k92"), Some("text 92"));
    }
    #[test]
    fn catalog_key_93() {
        let mut c = MessageCatalog::new("en");
        c.insert("k93", "text 93");
        assert_eq!(c.get("k93"), Some("text 93"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k93"), Some("text 93"));
    }
    #[test]
    fn catalog_key_94() {
        let mut c = MessageCatalog::new("en");
        c.insert("k94", "text 94");
        assert_eq!(c.get("k94"), Some("text 94"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k94"), Some("text 94"));
    }
    #[test]
    fn catalog_key_95() {
        let mut c = MessageCatalog::new("en");
        c.insert("k95", "text 95");
        assert_eq!(c.get("k95"), Some("text 95"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k95"), Some("text 95"));
    }
    #[test]
    fn catalog_key_96() {
        let mut c = MessageCatalog::new("en");
        c.insert("k96", "text 96");
        assert_eq!(c.get("k96"), Some("text 96"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k96"), Some("text 96"));
    }
    #[test]
    fn catalog_key_97() {
        let mut c = MessageCatalog::new("en");
        c.insert("k97", "text 97");
        assert_eq!(c.get("k97"), Some("text 97"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k97"), Some("text 97"));
    }
    #[test]
    fn catalog_key_98() {
        let mut c = MessageCatalog::new("en");
        c.insert("k98", "text 98");
        assert_eq!(c.get("k98"), Some("text 98"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k98"), Some("text 98"));
    }
    #[test]
    fn catalog_key_99() {
        let mut c = MessageCatalog::new("en");
        c.insert("k99", "text 99");
        assert_eq!(c.get("k99"), Some("text 99"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k99"), Some("text 99"));
    }
    #[test]
    fn catalog_key_100() {
        let mut c = MessageCatalog::new("en");
        c.insert("k100", "text 100");
        assert_eq!(c.get("k100"), Some("text 100"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k100"), Some("text 100"));
    }
    #[test]
    fn catalog_key_101() {
        let mut c = MessageCatalog::new("en");
        c.insert("k101", "text 101");
        assert_eq!(c.get("k101"), Some("text 101"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k101"), Some("text 101"));
    }
    #[test]
    fn catalog_key_102() {
        let mut c = MessageCatalog::new("en");
        c.insert("k102", "text 102");
        assert_eq!(c.get("k102"), Some("text 102"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k102"), Some("text 102"));
    }
    #[test]
    fn catalog_key_103() {
        let mut c = MessageCatalog::new("en");
        c.insert("k103", "text 103");
        assert_eq!(c.get("k103"), Some("text 103"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k103"), Some("text 103"));
    }
    #[test]
    fn catalog_key_104() {
        let mut c = MessageCatalog::new("en");
        c.insert("k104", "text 104");
        assert_eq!(c.get("k104"), Some("text 104"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k104"), Some("text 104"));
    }
    #[test]
    fn catalog_key_105() {
        let mut c = MessageCatalog::new("en");
        c.insert("k105", "text 105");
        assert_eq!(c.get("k105"), Some("text 105"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k105"), Some("text 105"));
    }
    #[test]
    fn catalog_key_106() {
        let mut c = MessageCatalog::new("en");
        c.insert("k106", "text 106");
        assert_eq!(c.get("k106"), Some("text 106"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k106"), Some("text 106"));
    }
    #[test]
    fn catalog_key_107() {
        let mut c = MessageCatalog::new("en");
        c.insert("k107", "text 107");
        assert_eq!(c.get("k107"), Some("text 107"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k107"), Some("text 107"));
    }
    #[test]
    fn catalog_key_108() {
        let mut c = MessageCatalog::new("en");
        c.insert("k108", "text 108");
        assert_eq!(c.get("k108"), Some("text 108"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k108"), Some("text 108"));
    }
    #[test]
    fn catalog_key_109() {
        let mut c = MessageCatalog::new("en");
        c.insert("k109", "text 109");
        assert_eq!(c.get("k109"), Some("text 109"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k109"), Some("text 109"));
    }
    #[test]
    fn catalog_key_110() {
        let mut c = MessageCatalog::new("en");
        c.insert("k110", "text 110");
        assert_eq!(c.get("k110"), Some("text 110"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k110"), Some("text 110"));
    }
    #[test]
    fn catalog_key_111() {
        let mut c = MessageCatalog::new("en");
        c.insert("k111", "text 111");
        assert_eq!(c.get("k111"), Some("text 111"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k111"), Some("text 111"));
    }
    #[test]
    fn catalog_key_112() {
        let mut c = MessageCatalog::new("en");
        c.insert("k112", "text 112");
        assert_eq!(c.get("k112"), Some("text 112"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k112"), Some("text 112"));
    }
    #[test]
    fn catalog_key_113() {
        let mut c = MessageCatalog::new("en");
        c.insert("k113", "text 113");
        assert_eq!(c.get("k113"), Some("text 113"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k113"), Some("text 113"));
    }
    #[test]
    fn catalog_key_114() {
        let mut c = MessageCatalog::new("en");
        c.insert("k114", "text 114");
        assert_eq!(c.get("k114"), Some("text 114"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k114"), Some("text 114"));
    }
    #[test]
    fn catalog_key_115() {
        let mut c = MessageCatalog::new("en");
        c.insert("k115", "text 115");
        assert_eq!(c.get("k115"), Some("text 115"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k115"), Some("text 115"));
    }
    #[test]
    fn catalog_key_116() {
        let mut c = MessageCatalog::new("en");
        c.insert("k116", "text 116");
        assert_eq!(c.get("k116"), Some("text 116"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k116"), Some("text 116"));
    }
    #[test]
    fn catalog_key_117() {
        let mut c = MessageCatalog::new("en");
        c.insert("k117", "text 117");
        assert_eq!(c.get("k117"), Some("text 117"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k117"), Some("text 117"));
    }
    #[test]
    fn catalog_key_118() {
        let mut c = MessageCatalog::new("en");
        c.insert("k118", "text 118");
        assert_eq!(c.get("k118"), Some("text 118"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k118"), Some("text 118"));
    }
    #[test]
    fn catalog_key_119() {
        let mut c = MessageCatalog::new("en");
        c.insert("k119", "text 119");
        assert_eq!(c.get("k119"), Some("text 119"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k119"), Some("text 119"));
    }
    #[test]
    fn catalog_key_120() {
        let mut c = MessageCatalog::new("en");
        c.insert("k120", "text 120");
        assert_eq!(c.get("k120"), Some("text 120"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k120"), Some("text 120"));
    }
    #[test]
    fn catalog_key_121() {
        let mut c = MessageCatalog::new("en");
        c.insert("k121", "text 121");
        assert_eq!(c.get("k121"), Some("text 121"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k121"), Some("text 121"));
    }
    #[test]
    fn catalog_key_122() {
        let mut c = MessageCatalog::new("en");
        c.insert("k122", "text 122");
        assert_eq!(c.get("k122"), Some("text 122"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k122"), Some("text 122"));
    }
    #[test]
    fn catalog_key_123() {
        let mut c = MessageCatalog::new("en");
        c.insert("k123", "text 123");
        assert_eq!(c.get("k123"), Some("text 123"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k123"), Some("text 123"));
    }
    #[test]
    fn catalog_key_124() {
        let mut c = MessageCatalog::new("en");
        c.insert("k124", "text 124");
        assert_eq!(c.get("k124"), Some("text 124"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k124"), Some("text 124"));
    }
    #[test]
    fn catalog_key_125() {
        let mut c = MessageCatalog::new("en");
        c.insert("k125", "text 125");
        assert_eq!(c.get("k125"), Some("text 125"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k125"), Some("text 125"));
    }
    #[test]
    fn catalog_key_126() {
        let mut c = MessageCatalog::new("en");
        c.insert("k126", "text 126");
        assert_eq!(c.get("k126"), Some("text 126"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k126"), Some("text 126"));
    }
    #[test]
    fn catalog_key_127() {
        let mut c = MessageCatalog::new("en");
        c.insert("k127", "text 127");
        assert_eq!(c.get("k127"), Some("text 127"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k127"), Some("text 127"));
    }
    #[test]
    fn catalog_key_128() {
        let mut c = MessageCatalog::new("en");
        c.insert("k128", "text 128");
        assert_eq!(c.get("k128"), Some("text 128"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k128"), Some("text 128"));
    }
    #[test]
    fn catalog_key_129() {
        let mut c = MessageCatalog::new("en");
        c.insert("k129", "text 129");
        assert_eq!(c.get("k129"), Some("text 129"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k129"), Some("text 129"));
    }
    #[test]
    fn catalog_key_130() {
        let mut c = MessageCatalog::new("en");
        c.insert("k130", "text 130");
        assert_eq!(c.get("k130"), Some("text 130"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k130"), Some("text 130"));
    }
    #[test]
    fn catalog_key_131() {
        let mut c = MessageCatalog::new("en");
        c.insert("k131", "text 131");
        assert_eq!(c.get("k131"), Some("text 131"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k131"), Some("text 131"));
    }
    #[test]
    fn catalog_key_132() {
        let mut c = MessageCatalog::new("en");
        c.insert("k132", "text 132");
        assert_eq!(c.get("k132"), Some("text 132"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k132"), Some("text 132"));
    }
    #[test]
    fn catalog_key_133() {
        let mut c = MessageCatalog::new("en");
        c.insert("k133", "text 133");
        assert_eq!(c.get("k133"), Some("text 133"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k133"), Some("text 133"));
    }
    #[test]
    fn catalog_key_134() {
        let mut c = MessageCatalog::new("en");
        c.insert("k134", "text 134");
        assert_eq!(c.get("k134"), Some("text 134"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k134"), Some("text 134"));
    }
    #[test]
    fn catalog_key_135() {
        let mut c = MessageCatalog::new("en");
        c.insert("k135", "text 135");
        assert_eq!(c.get("k135"), Some("text 135"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k135"), Some("text 135"));
    }
    #[test]
    fn catalog_key_136() {
        let mut c = MessageCatalog::new("en");
        c.insert("k136", "text 136");
        assert_eq!(c.get("k136"), Some("text 136"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k136"), Some("text 136"));
    }
    #[test]
    fn catalog_key_137() {
        let mut c = MessageCatalog::new("en");
        c.insert("k137", "text 137");
        assert_eq!(c.get("k137"), Some("text 137"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k137"), Some("text 137"));
    }
    #[test]
    fn catalog_key_138() {
        let mut c = MessageCatalog::new("en");
        c.insert("k138", "text 138");
        assert_eq!(c.get("k138"), Some("text 138"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k138"), Some("text 138"));
    }
    #[test]
    fn catalog_key_139() {
        let mut c = MessageCatalog::new("en");
        c.insert("k139", "text 139");
        assert_eq!(c.get("k139"), Some("text 139"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k139"), Some("text 139"));
    }
    #[test]
    fn catalog_key_140() {
        let mut c = MessageCatalog::new("en");
        c.insert("k140", "text 140");
        assert_eq!(c.get("k140"), Some("text 140"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k140"), Some("text 140"));
    }
    #[test]
    fn catalog_key_141() {
        let mut c = MessageCatalog::new("en");
        c.insert("k141", "text 141");
        assert_eq!(c.get("k141"), Some("text 141"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k141"), Some("text 141"));
    }
    #[test]
    fn catalog_key_142() {
        let mut c = MessageCatalog::new("en");
        c.insert("k142", "text 142");
        assert_eq!(c.get("k142"), Some("text 142"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k142"), Some("text 142"));
    }
    #[test]
    fn catalog_key_143() {
        let mut c = MessageCatalog::new("en");
        c.insert("k143", "text 143");
        assert_eq!(c.get("k143"), Some("text 143"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k143"), Some("text 143"));
    }
    #[test]
    fn catalog_key_144() {
        let mut c = MessageCatalog::new("en");
        c.insert("k144", "text 144");
        assert_eq!(c.get("k144"), Some("text 144"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k144"), Some("text 144"));
    }
    #[test]
    fn catalog_key_145() {
        let mut c = MessageCatalog::new("en");
        c.insert("k145", "text 145");
        assert_eq!(c.get("k145"), Some("text 145"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k145"), Some("text 145"));
    }
    #[test]
    fn catalog_key_146() {
        let mut c = MessageCatalog::new("en");
        c.insert("k146", "text 146");
        assert_eq!(c.get("k146"), Some("text 146"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k146"), Some("text 146"));
    }
    #[test]
    fn catalog_key_147() {
        let mut c = MessageCatalog::new("en");
        c.insert("k147", "text 147");
        assert_eq!(c.get("k147"), Some("text 147"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k147"), Some("text 147"));
    }
    #[test]
    fn catalog_key_148() {
        let mut c = MessageCatalog::new("en");
        c.insert("k148", "text 148");
        assert_eq!(c.get("k148"), Some("text 148"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k148"), Some("text 148"));
    }
    #[test]
    fn catalog_key_149() {
        let mut c = MessageCatalog::new("en");
        c.insert("k149", "text 149");
        assert_eq!(c.get("k149"), Some("text 149"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k149"), Some("text 149"));
    }
    #[test]
    fn catalog_key_150() {
        let mut c = MessageCatalog::new("en");
        c.insert("k150", "text 150");
        assert_eq!(c.get("k150"), Some("text 150"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k150"), Some("text 150"));
    }
    #[test]
    fn catalog_key_151() {
        let mut c = MessageCatalog::new("en");
        c.insert("k151", "text 151");
        assert_eq!(c.get("k151"), Some("text 151"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k151"), Some("text 151"));
    }
    #[test]
    fn catalog_key_152() {
        let mut c = MessageCatalog::new("en");
        c.insert("k152", "text 152");
        assert_eq!(c.get("k152"), Some("text 152"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k152"), Some("text 152"));
    }
    #[test]
    fn catalog_key_153() {
        let mut c = MessageCatalog::new("en");
        c.insert("k153", "text 153");
        assert_eq!(c.get("k153"), Some("text 153"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k153"), Some("text 153"));
    }
    #[test]
    fn catalog_key_154() {
        let mut c = MessageCatalog::new("en");
        c.insert("k154", "text 154");
        assert_eq!(c.get("k154"), Some("text 154"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k154"), Some("text 154"));
    }
    #[test]
    fn catalog_key_155() {
        let mut c = MessageCatalog::new("en");
        c.insert("k155", "text 155");
        assert_eq!(c.get("k155"), Some("text 155"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k155"), Some("text 155"));
    }
    #[test]
    fn catalog_key_156() {
        let mut c = MessageCatalog::new("en");
        c.insert("k156", "text 156");
        assert_eq!(c.get("k156"), Some("text 156"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k156"), Some("text 156"));
    }
    #[test]
    fn catalog_key_157() {
        let mut c = MessageCatalog::new("en");
        c.insert("k157", "text 157");
        assert_eq!(c.get("k157"), Some("text 157"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k157"), Some("text 157"));
    }
    #[test]
    fn catalog_key_158() {
        let mut c = MessageCatalog::new("en");
        c.insert("k158", "text 158");
        assert_eq!(c.get("k158"), Some("text 158"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k158"), Some("text 158"));
    }
    #[test]
    fn catalog_key_159() {
        let mut c = MessageCatalog::new("en");
        c.insert("k159", "text 159");
        assert_eq!(c.get("k159"), Some("text 159"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k159"), Some("text 159"));
    }
    #[test]
    fn catalog_key_160() {
        let mut c = MessageCatalog::new("en");
        c.insert("k160", "text 160");
        assert_eq!(c.get("k160"), Some("text 160"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k160"), Some("text 160"));
    }
    #[test]
    fn catalog_key_161() {
        let mut c = MessageCatalog::new("en");
        c.insert("k161", "text 161");
        assert_eq!(c.get("k161"), Some("text 161"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k161"), Some("text 161"));
    }
    #[test]
    fn catalog_key_162() {
        let mut c = MessageCatalog::new("en");
        c.insert("k162", "text 162");
        assert_eq!(c.get("k162"), Some("text 162"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k162"), Some("text 162"));
    }
    #[test]
    fn catalog_key_163() {
        let mut c = MessageCatalog::new("en");
        c.insert("k163", "text 163");
        assert_eq!(c.get("k163"), Some("text 163"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k163"), Some("text 163"));
    }
    #[test]
    fn catalog_key_164() {
        let mut c = MessageCatalog::new("en");
        c.insert("k164", "text 164");
        assert_eq!(c.get("k164"), Some("text 164"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k164"), Some("text 164"));
    }
    #[test]
    fn catalog_key_165() {
        let mut c = MessageCatalog::new("en");
        c.insert("k165", "text 165");
        assert_eq!(c.get("k165"), Some("text 165"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k165"), Some("text 165"));
    }
    #[test]
    fn catalog_key_166() {
        let mut c = MessageCatalog::new("en");
        c.insert("k166", "text 166");
        assert_eq!(c.get("k166"), Some("text 166"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k166"), Some("text 166"));
    }
    #[test]
    fn catalog_key_167() {
        let mut c = MessageCatalog::new("en");
        c.insert("k167", "text 167");
        assert_eq!(c.get("k167"), Some("text 167"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k167"), Some("text 167"));
    }
    #[test]
    fn catalog_key_168() {
        let mut c = MessageCatalog::new("en");
        c.insert("k168", "text 168");
        assert_eq!(c.get("k168"), Some("text 168"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k168"), Some("text 168"));
    }
    #[test]
    fn catalog_key_169() {
        let mut c = MessageCatalog::new("en");
        c.insert("k169", "text 169");
        assert_eq!(c.get("k169"), Some("text 169"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k169"), Some("text 169"));
    }
    #[test]
    fn catalog_key_170() {
        let mut c = MessageCatalog::new("en");
        c.insert("k170", "text 170");
        assert_eq!(c.get("k170"), Some("text 170"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k170"), Some("text 170"));
    }
    #[test]
    fn catalog_key_171() {
        let mut c = MessageCatalog::new("en");
        c.insert("k171", "text 171");
        assert_eq!(c.get("k171"), Some("text 171"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k171"), Some("text 171"));
    }
    #[test]
    fn catalog_key_172() {
        let mut c = MessageCatalog::new("en");
        c.insert("k172", "text 172");
        assert_eq!(c.get("k172"), Some("text 172"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k172"), Some("text 172"));
    }
    #[test]
    fn catalog_key_173() {
        let mut c = MessageCatalog::new("en");
        c.insert("k173", "text 173");
        assert_eq!(c.get("k173"), Some("text 173"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k173"), Some("text 173"));
    }
    #[test]
    fn catalog_key_174() {
        let mut c = MessageCatalog::new("en");
        c.insert("k174", "text 174");
        assert_eq!(c.get("k174"), Some("text 174"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k174"), Some("text 174"));
    }
    #[test]
    fn catalog_key_175() {
        let mut c = MessageCatalog::new("en");
        c.insert("k175", "text 175");
        assert_eq!(c.get("k175"), Some("text 175"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k175"), Some("text 175"));
    }
    #[test]
    fn catalog_key_176() {
        let mut c = MessageCatalog::new("en");
        c.insert("k176", "text 176");
        assert_eq!(c.get("k176"), Some("text 176"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k176"), Some("text 176"));
    }
    #[test]
    fn catalog_key_177() {
        let mut c = MessageCatalog::new("en");
        c.insert("k177", "text 177");
        assert_eq!(c.get("k177"), Some("text 177"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k177"), Some("text 177"));
    }
    #[test]
    fn catalog_key_178() {
        let mut c = MessageCatalog::new("en");
        c.insert("k178", "text 178");
        assert_eq!(c.get("k178"), Some("text 178"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k178"), Some("text 178"));
    }
    #[test]
    fn catalog_key_179() {
        let mut c = MessageCatalog::new("en");
        c.insert("k179", "text 179");
        assert_eq!(c.get("k179"), Some("text 179"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k179"), Some("text 179"));
    }
    #[test]
    fn catalog_key_180() {
        let mut c = MessageCatalog::new("en");
        c.insert("k180", "text 180");
        assert_eq!(c.get("k180"), Some("text 180"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k180"), Some("text 180"));
    }
    #[test]
    fn catalog_key_181() {
        let mut c = MessageCatalog::new("en");
        c.insert("k181", "text 181");
        assert_eq!(c.get("k181"), Some("text 181"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k181"), Some("text 181"));
    }
    #[test]
    fn catalog_key_182() {
        let mut c = MessageCatalog::new("en");
        c.insert("k182", "text 182");
        assert_eq!(c.get("k182"), Some("text 182"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k182"), Some("text 182"));
    }
    #[test]
    fn catalog_key_183() {
        let mut c = MessageCatalog::new("en");
        c.insert("k183", "text 183");
        assert_eq!(c.get("k183"), Some("text 183"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k183"), Some("text 183"));
    }
    #[test]
    fn catalog_key_184() {
        let mut c = MessageCatalog::new("en");
        c.insert("k184", "text 184");
        assert_eq!(c.get("k184"), Some("text 184"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k184"), Some("text 184"));
    }
    #[test]
    fn catalog_key_185() {
        let mut c = MessageCatalog::new("en");
        c.insert("k185", "text 185");
        assert_eq!(c.get("k185"), Some("text 185"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k185"), Some("text 185"));
    }
    #[test]
    fn catalog_key_186() {
        let mut c = MessageCatalog::new("en");
        c.insert("k186", "text 186");
        assert_eq!(c.get("k186"), Some("text 186"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k186"), Some("text 186"));
    }
    #[test]
    fn catalog_key_187() {
        let mut c = MessageCatalog::new("en");
        c.insert("k187", "text 187");
        assert_eq!(c.get("k187"), Some("text 187"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k187"), Some("text 187"));
    }
    #[test]
    fn catalog_key_188() {
        let mut c = MessageCatalog::new("en");
        c.insert("k188", "text 188");
        assert_eq!(c.get("k188"), Some("text 188"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k188"), Some("text 188"));
    }
    #[test]
    fn catalog_key_189() {
        let mut c = MessageCatalog::new("en");
        c.insert("k189", "text 189");
        assert_eq!(c.get("k189"), Some("text 189"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k189"), Some("text 189"));
    }
    #[test]
    fn catalog_key_190() {
        let mut c = MessageCatalog::new("en");
        c.insert("k190", "text 190");
        assert_eq!(c.get("k190"), Some("text 190"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k190"), Some("text 190"));
    }
    #[test]
    fn catalog_key_191() {
        let mut c = MessageCatalog::new("en");
        c.insert("k191", "text 191");
        assert_eq!(c.get("k191"), Some("text 191"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k191"), Some("text 191"));
    }
    #[test]
    fn catalog_key_192() {
        let mut c = MessageCatalog::new("en");
        c.insert("k192", "text 192");
        assert_eq!(c.get("k192"), Some("text 192"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k192"), Some("text 192"));
    }
    #[test]
    fn catalog_key_193() {
        let mut c = MessageCatalog::new("en");
        c.insert("k193", "text 193");
        assert_eq!(c.get("k193"), Some("text 193"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k193"), Some("text 193"));
    }
    #[test]
    fn catalog_key_194() {
        let mut c = MessageCatalog::new("en");
        c.insert("k194", "text 194");
        assert_eq!(c.get("k194"), Some("text 194"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k194"), Some("text 194"));
    }
    #[test]
    fn catalog_key_195() {
        let mut c = MessageCatalog::new("en");
        c.insert("k195", "text 195");
        assert_eq!(c.get("k195"), Some("text 195"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k195"), Some("text 195"));
    }
    #[test]
    fn catalog_key_196() {
        let mut c = MessageCatalog::new("en");
        c.insert("k196", "text 196");
        assert_eq!(c.get("k196"), Some("text 196"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k196"), Some("text 196"));
    }
    #[test]
    fn catalog_key_197() {
        let mut c = MessageCatalog::new("en");
        c.insert("k197", "text 197");
        assert_eq!(c.get("k197"), Some("text 197"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k197"), Some("text 197"));
    }
    #[test]
    fn catalog_key_198() {
        let mut c = MessageCatalog::new("en");
        c.insert("k198", "text 198");
        assert_eq!(c.get("k198"), Some("text 198"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k198"), Some("text 198"));
    }
    #[test]
    fn catalog_key_199() {
        let mut c = MessageCatalog::new("en");
        c.insert("k199", "text 199");
        assert_eq!(c.get("k199"), Some("text 199"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k199"), Some("text 199"));
    }
    #[test]
    fn catalog_key_200() {
        let mut c = MessageCatalog::new("en");
        c.insert("k200", "text 200");
        assert_eq!(c.get("k200"), Some("text 200"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k200"), Some("text 200"));
    }
    #[test]
    fn catalog_key_201() {
        let mut c = MessageCatalog::new("en");
        c.insert("k201", "text 201");
        assert_eq!(c.get("k201"), Some("text 201"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k201"), Some("text 201"));
    }
    #[test]
    fn catalog_key_202() {
        let mut c = MessageCatalog::new("en");
        c.insert("k202", "text 202");
        assert_eq!(c.get("k202"), Some("text 202"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k202"), Some("text 202"));
    }
    #[test]
    fn catalog_key_203() {
        let mut c = MessageCatalog::new("en");
        c.insert("k203", "text 203");
        assert_eq!(c.get("k203"), Some("text 203"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k203"), Some("text 203"));
    }
    #[test]
    fn catalog_key_204() {
        let mut c = MessageCatalog::new("en");
        c.insert("k204", "text 204");
        assert_eq!(c.get("k204"), Some("text 204"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k204"), Some("text 204"));
    }
    #[test]
    fn catalog_key_205() {
        let mut c = MessageCatalog::new("en");
        c.insert("k205", "text 205");
        assert_eq!(c.get("k205"), Some("text 205"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k205"), Some("text 205"));
    }
    #[test]
    fn catalog_key_206() {
        let mut c = MessageCatalog::new("en");
        c.insert("k206", "text 206");
        assert_eq!(c.get("k206"), Some("text 206"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k206"), Some("text 206"));
    }
    #[test]
    fn catalog_key_207() {
        let mut c = MessageCatalog::new("en");
        c.insert("k207", "text 207");
        assert_eq!(c.get("k207"), Some("text 207"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k207"), Some("text 207"));
    }
    #[test]
    fn catalog_key_208() {
        let mut c = MessageCatalog::new("en");
        c.insert("k208", "text 208");
        assert_eq!(c.get("k208"), Some("text 208"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k208"), Some("text 208"));
    }
    #[test]
    fn catalog_key_209() {
        let mut c = MessageCatalog::new("en");
        c.insert("k209", "text 209");
        assert_eq!(c.get("k209"), Some("text 209"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k209"), Some("text 209"));
    }
    #[test]
    fn catalog_key_210() {
        let mut c = MessageCatalog::new("en");
        c.insert("k210", "text 210");
        assert_eq!(c.get("k210"), Some("text 210"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k210"), Some("text 210"));
    }
    #[test]
    fn catalog_key_211() {
        let mut c = MessageCatalog::new("en");
        c.insert("k211", "text 211");
        assert_eq!(c.get("k211"), Some("text 211"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k211"), Some("text 211"));
    }
    #[test]
    fn catalog_key_212() {
        let mut c = MessageCatalog::new("en");
        c.insert("k212", "text 212");
        assert_eq!(c.get("k212"), Some("text 212"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k212"), Some("text 212"));
    }
    #[test]
    fn catalog_key_213() {
        let mut c = MessageCatalog::new("en");
        c.insert("k213", "text 213");
        assert_eq!(c.get("k213"), Some("text 213"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k213"), Some("text 213"));
    }
    #[test]
    fn catalog_key_214() {
        let mut c = MessageCatalog::new("en");
        c.insert("k214", "text 214");
        assert_eq!(c.get("k214"), Some("text 214"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k214"), Some("text 214"));
    }
    #[test]
    fn catalog_key_215() {
        let mut c = MessageCatalog::new("en");
        c.insert("k215", "text 215");
        assert_eq!(c.get("k215"), Some("text 215"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k215"), Some("text 215"));
    }
    #[test]
    fn catalog_key_216() {
        let mut c = MessageCatalog::new("en");
        c.insert("k216", "text 216");
        assert_eq!(c.get("k216"), Some("text 216"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k216"), Some("text 216"));
    }
    #[test]
    fn catalog_key_217() {
        let mut c = MessageCatalog::new("en");
        c.insert("k217", "text 217");
        assert_eq!(c.get("k217"), Some("text 217"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k217"), Some("text 217"));
    }
    #[test]
    fn catalog_key_218() {
        let mut c = MessageCatalog::new("en");
        c.insert("k218", "text 218");
        assert_eq!(c.get("k218"), Some("text 218"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k218"), Some("text 218"));
    }
    #[test]
    fn catalog_key_219() {
        let mut c = MessageCatalog::new("en");
        c.insert("k219", "text 219");
        assert_eq!(c.get("k219"), Some("text 219"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k219"), Some("text 219"));
    }
    #[test]
    fn catalog_key_220() {
        let mut c = MessageCatalog::new("en");
        c.insert("k220", "text 220");
        assert_eq!(c.get("k220"), Some("text 220"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k220"), Some("text 220"));
    }
    #[test]
    fn catalog_key_221() {
        let mut c = MessageCatalog::new("en");
        c.insert("k221", "text 221");
        assert_eq!(c.get("k221"), Some("text 221"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k221"), Some("text 221"));
    }
    #[test]
    fn catalog_key_222() {
        let mut c = MessageCatalog::new("en");
        c.insert("k222", "text 222");
        assert_eq!(c.get("k222"), Some("text 222"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k222"), Some("text 222"));
    }
    #[test]
    fn catalog_key_223() {
        let mut c = MessageCatalog::new("en");
        c.insert("k223", "text 223");
        assert_eq!(c.get("k223"), Some("text 223"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k223"), Some("text 223"));
    }
    #[test]
    fn catalog_key_224() {
        let mut c = MessageCatalog::new("en");
        c.insert("k224", "text 224");
        assert_eq!(c.get("k224"), Some("text 224"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k224"), Some("text 224"));
    }
    #[test]
    fn catalog_key_225() {
        let mut c = MessageCatalog::new("en");
        c.insert("k225", "text 225");
        assert_eq!(c.get("k225"), Some("text 225"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k225"), Some("text 225"));
    }
    #[test]
    fn catalog_key_226() {
        let mut c = MessageCatalog::new("en");
        c.insert("k226", "text 226");
        assert_eq!(c.get("k226"), Some("text 226"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k226"), Some("text 226"));
    }
    #[test]
    fn catalog_key_227() {
        let mut c = MessageCatalog::new("en");
        c.insert("k227", "text 227");
        assert_eq!(c.get("k227"), Some("text 227"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k227"), Some("text 227"));
    }
    #[test]
    fn catalog_key_228() {
        let mut c = MessageCatalog::new("en");
        c.insert("k228", "text 228");
        assert_eq!(c.get("k228"), Some("text 228"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k228"), Some("text 228"));
    }
    #[test]
    fn catalog_key_229() {
        let mut c = MessageCatalog::new("en");
        c.insert("k229", "text 229");
        assert_eq!(c.get("k229"), Some("text 229"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k229"), Some("text 229"));
    }
    #[test]
    fn catalog_key_230() {
        let mut c = MessageCatalog::new("en");
        c.insert("k230", "text 230");
        assert_eq!(c.get("k230"), Some("text 230"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k230"), Some("text 230"));
    }
    #[test]
    fn catalog_key_231() {
        let mut c = MessageCatalog::new("en");
        c.insert("k231", "text 231");
        assert_eq!(c.get("k231"), Some("text 231"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k231"), Some("text 231"));
    }
    #[test]
    fn catalog_key_232() {
        let mut c = MessageCatalog::new("en");
        c.insert("k232", "text 232");
        assert_eq!(c.get("k232"), Some("text 232"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k232"), Some("text 232"));
    }
    #[test]
    fn catalog_key_233() {
        let mut c = MessageCatalog::new("en");
        c.insert("k233", "text 233");
        assert_eq!(c.get("k233"), Some("text 233"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k233"), Some("text 233"));
    }
    #[test]
    fn catalog_key_234() {
        let mut c = MessageCatalog::new("en");
        c.insert("k234", "text 234");
        assert_eq!(c.get("k234"), Some("text 234"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k234"), Some("text 234"));
    }
    #[test]
    fn catalog_key_235() {
        let mut c = MessageCatalog::new("en");
        c.insert("k235", "text 235");
        assert_eq!(c.get("k235"), Some("text 235"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k235"), Some("text 235"));
    }
    #[test]
    fn catalog_key_236() {
        let mut c = MessageCatalog::new("en");
        c.insert("k236", "text 236");
        assert_eq!(c.get("k236"), Some("text 236"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k236"), Some("text 236"));
    }
    #[test]
    fn catalog_key_237() {
        let mut c = MessageCatalog::new("en");
        c.insert("k237", "text 237");
        assert_eq!(c.get("k237"), Some("text 237"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k237"), Some("text 237"));
    }
    #[test]
    fn catalog_key_238() {
        let mut c = MessageCatalog::new("en");
        c.insert("k238", "text 238");
        assert_eq!(c.get("k238"), Some("text 238"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k238"), Some("text 238"));
    }
    #[test]
    fn catalog_key_239() {
        let mut c = MessageCatalog::new("en");
        c.insert("k239", "text 239");
        assert_eq!(c.get("k239"), Some("text 239"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k239"), Some("text 239"));
    }
    #[test]
    fn catalog_key_240() {
        let mut c = MessageCatalog::new("en");
        c.insert("k240", "text 240");
        assert_eq!(c.get("k240"), Some("text 240"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k240"), Some("text 240"));
    }
    #[test]
    fn catalog_key_241() {
        let mut c = MessageCatalog::new("en");
        c.insert("k241", "text 241");
        assert_eq!(c.get("k241"), Some("text 241"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k241"), Some("text 241"));
    }
    #[test]
    fn catalog_key_242() {
        let mut c = MessageCatalog::new("en");
        c.insert("k242", "text 242");
        assert_eq!(c.get("k242"), Some("text 242"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k242"), Some("text 242"));
    }
    #[test]
    fn catalog_key_243() {
        let mut c = MessageCatalog::new("en");
        c.insert("k243", "text 243");
        assert_eq!(c.get("k243"), Some("text 243"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k243"), Some("text 243"));
    }
    #[test]
    fn catalog_key_244() {
        let mut c = MessageCatalog::new("en");
        c.insert("k244", "text 244");
        assert_eq!(c.get("k244"), Some("text 244"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k244"), Some("text 244"));
    }
    #[test]
    fn catalog_key_245() {
        let mut c = MessageCatalog::new("en");
        c.insert("k245", "text 245");
        assert_eq!(c.get("k245"), Some("text 245"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k245"), Some("text 245"));
    }
    #[test]
    fn catalog_key_246() {
        let mut c = MessageCatalog::new("en");
        c.insert("k246", "text 246");
        assert_eq!(c.get("k246"), Some("text 246"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k246"), Some("text 246"));
    }
    #[test]
    fn catalog_key_247() {
        let mut c = MessageCatalog::new("en");
        c.insert("k247", "text 247");
        assert_eq!(c.get("k247"), Some("text 247"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k247"), Some("text 247"));
    }
    #[test]
    fn catalog_key_248() {
        let mut c = MessageCatalog::new("en");
        c.insert("k248", "text 248");
        assert_eq!(c.get("k248"), Some("text 248"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k248"), Some("text 248"));
    }
    #[test]
    fn catalog_key_249() {
        let mut c = MessageCatalog::new("en");
        c.insert("k249", "text 249");
        assert_eq!(c.get("k249"), Some("text 249"));
        let j = c.to_json().unwrap();
        let c2 = MessageCatalog::from_json(&j).unwrap();
        assert_eq!(c2.get("k249"), Some("text 249"));
    }
}
