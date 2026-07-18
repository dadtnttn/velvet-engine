//! In-story glossary / codex of unlocked terms.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Glossary errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GlossaryError {
    /// Term id unknown.
    #[error("glossary term not found: {0}")]
    NotFound(String),
}

/// One glossary term definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryTerm {
    /// Stable id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Body text (markdown or plain).
    pub body: String,
    /// Optional category tag.
    #[serde(default)]
    pub category: String,
    /// Optional search aliases.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Whether unlocked by default.
    #[serde(default)]
    pub unlocked_by_default: bool,
}

impl GlossaryTerm {
    /// Create a term.
    pub fn new(id: impl Into<String>, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: body.into(),
            category: String::new(),
            aliases: Vec::new(),
            unlocked_by_default: false,
        }
    }

    /// Builder: category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Builder: aliases.
    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    /// Builder: default unlocked.
    pub fn default_unlocked(mut self) -> Self {
        self.unlocked_by_default = true;
        self
    }
}

/// Glossary catalog + unlock state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Glossary {
    /// Terms by id.
    #[serde(default)]
    pub terms: BTreeMap<String, GlossaryTerm>,
    /// Unlocked ids.
    #[serde(default)]
    pub unlocked: BTreeSet<String>,
}

impl Glossary {
    /// Empty glossary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a term.
    pub fn register(&mut self, term: GlossaryTerm) {
        let id = term.id.clone();
        let auto = term.unlocked_by_default;
        self.terms.insert(id.clone(), term);
        if auto {
            self.unlocked.insert(id);
        }
    }

    /// Register many terms.
    pub fn register_all<I>(&mut self, terms: I)
    where
        I: IntoIterator<Item = GlossaryTerm>,
    {
        for t in terms {
            self.register(t);
        }
    }

    /// Unlock by id. Returns whether newly unlocked.
    pub fn unlock(&mut self, id: &str) -> Result<bool, GlossaryError> {
        if !self.terms.contains_key(id) {
            return Err(GlossaryError::NotFound(id.into()));
        }
        Ok(self.unlocked.insert(id.into()))
    }

    /// Unlock if known (no error on unknown).
    pub fn unlock_if_known(&mut self, id: &str) -> bool {
        self.terms.contains_key(id) && self.unlocked.insert(id.into())
    }

    /// Whether unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }

    /// Lookup term if unlocked (UI-safe).
    pub fn lookup(&self, id: &str) -> Option<&GlossaryTerm> {
        if self.is_unlocked(id) {
            self.terms.get(id)
        } else {
            None
        }
    }

    /// Lookup regardless of unlock (authoring / debug).
    pub fn get_raw(&self, id: &str) -> Option<&GlossaryTerm> {
        self.terms.get(id)
    }

    /// Case-insensitive search over unlocked titles, bodies, and aliases.
    /// Returns matching term ids sorted by title.
    pub fn search(&self, query: &str) -> Vec<&GlossaryTerm> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.list_unlocked();
        }
        let mut out: Vec<&GlossaryTerm> = self
            .unlocked
            .iter()
            .filter_map(|id| self.terms.get(id))
            .filter(|t| {
                t.title.to_lowercase().contains(&q)
                    || t.body.to_lowercase().contains(&q)
                    || t.category.to_lowercase().contains(&q)
                    || t.aliases.iter().any(|a| a.to_lowercase().contains(&q))
            })
            .collect();
        out.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// All unlocked terms sorted by title.
    pub fn list_unlocked(&self) -> Vec<&GlossaryTerm> {
        let mut out: Vec<&GlossaryTerm> = self
            .unlocked
            .iter()
            .filter_map(|id| self.terms.get(id))
            .collect();
        out.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// Unlocked terms in a category.
    pub fn list_by_category(&self, category: &str) -> Vec<&GlossaryTerm> {
        self.list_unlocked()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Unlock progress (unlocked, total).
    pub fn progress(&self) -> (usize, usize) {
        (self.unlocked.len(), self.terms.len())
    }

    /// Export unlock set for saves.
    pub fn unlock_set(&self) -> BTreeSet<String> {
        self.unlocked.clone()
    }

    /// Merge unlocks from a save.
    pub fn apply_unlock_set(&mut self, set: impl IntoIterator<Item = String>) {
        for id in set {
            self.unlocked.insert(id);
        }
    }

    /// Clear unlocks and restore defaults.
    pub fn clear_unlocks(&mut self) {
        self.unlocked.clear();
        for t in self.terms.values() {
            if t.unlocked_by_default {
                self.unlocked.insert(t.id.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Glossary {
        let mut g = Glossary::new();
        g.register(
            GlossaryTerm::new("aether", "Aether", "A luminous energy used by mages.")
                .with_category("magic")
                .with_aliases(["ether", "mana-light"])
                .default_unlocked(),
        );
        g.register(
            GlossaryTerm::new("velvet", "Velvet Order", "A secretive guild.")
                .with_category("factions")
                .with_aliases(["order"]),
        );
        g.register(
            GlossaryTerm::new("harbor", "Old Harbor", "Coastal trade hub.").with_category("places"),
        );
        g
    }

    #[test]
    fn unlock_and_lookup() {
        let mut g = sample();
        assert!(g.lookup("aether").is_some());
        assert!(g.lookup("velvet").is_none());
        assert!(g.unlock("velvet").unwrap());
        assert_eq!(g.lookup("velvet").unwrap().title, "Velvet Order");
        assert!(matches!(g.unlock("nope"), Err(GlossaryError::NotFound(_))));
    }

    #[test]
    fn search_titles_aliases_body() {
        let mut g = sample();
        g.unlock("velvet").unwrap();
        g.unlock("harbor").unwrap();
        let by_title = g.search("Harbor");
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].id, "harbor");
        let by_alias = g.search("ether");
        assert_eq!(by_alias.len(), 1);
        assert_eq!(by_alias[0].id, "aether");
        let by_body = g.search("guild");
        assert_eq!(by_body.len(), 1);
        assert_eq!(by_body[0].id, "velvet");
        let empty = g.search("   ");
        assert_eq!(empty.len(), 3);
    }

    #[test]
    fn category_and_progress() {
        let mut g = sample();
        g.unlock("velvet").unwrap();
        let factions = g.list_by_category("factions");
        assert_eq!(factions.len(), 1);
        assert_eq!(g.progress(), (2, 3));
    }

    #[test]
    fn unlock_set_roundtrip() {
        let mut g = sample();
        g.unlock("harbor").unwrap();
        let set = g.unlock_set();
        let mut g2 = sample();
        g2.clear_unlocks();
        g2.apply_unlock_set(set);
        assert!(g2.is_unlocked("aether"));
        assert!(g2.is_unlocked("harbor"));
        assert!(!g2.is_unlocked("velvet"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut g = sample();
        g.unlock("velvet").unwrap();
        let json = serde_json::to_string_pretty(&g).unwrap();
        let back: Glossary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }
}
