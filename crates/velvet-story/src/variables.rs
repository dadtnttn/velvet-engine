//! Story variable store with per-play / global / persistent layers.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::value::StoryValue;

/// Named variable storage (order-preserving for stable dumps).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryVariables {
    /// Per-playthrough variables (saved in slot).
    pub play: IndexMap<String, StoryValue>,
    /// Session globals (optional persistence separate).
    pub global: IndexMap<String, StoryValue>,
    /// Cross-playthrough persistent flags (achievements, gallery unlocks).
    pub persistent: IndexMap<String, StoryValue>,
}

impl StoryVariables {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get play variable.
    pub fn get(&self, name: &str) -> StoryValue {
        self.play
            .get(name)
            .cloned()
            .or_else(|| self.global.get(name).cloned())
            .or_else(|| self.persistent.get(name).cloned())
            .unwrap_or(StoryValue::Null)
    }

    /// Set play variable.
    pub fn set(&mut self, name: impl Into<String>, value: StoryValue) {
        self.play.insert(name.into(), value);
    }

    /// Set persistent.
    pub fn set_persistent(&mut self, name: impl Into<String>, value: StoryValue) {
        self.persistent.insert(name.into(), value);
    }

    /// Set global.
    pub fn set_global(&mut self, name: impl Into<String>, value: StoryValue) {
        self.global.insert(name.into(), value);
    }

    /// Integer get with default.
    pub fn get_int(&self, name: &str, default: i64) -> i64 {
        self.get(name).as_i64().unwrap_or(default)
    }

    /// Apply `+=` / `-=` / `=` for integers/floats.
    pub fn apply_assign(&mut self, name: &str, op: AssignOp, rhs: StoryValue) {
        match op {
            AssignOp::Set => self.set(name, rhs),
            AssignOp::Add => {
                let cur = self.get(name);
                if let (Some(a), Some(b)) = (cur.as_i64(), rhs.as_i64()) {
                    self.set(name, StoryValue::Int(a + b));
                } else if let (Some(a), Some(b)) = (cur.as_f64(), rhs.as_f64()) {
                    self.set(name, StoryValue::Float(a + b));
                } else if let (StoryValue::String(s), StoryValue::String(t)) = (cur, rhs) {
                    self.set(name, StoryValue::String(format!("{s}{t}")));
                }
            }
            AssignOp::Sub => {
                let cur = self.get(name);
                if let (Some(a), Some(b)) = (cur.as_i64(), rhs.as_i64()) {
                    self.set(name, StoryValue::Int(a - b));
                } else if let (Some(a), Some(b)) = (cur.as_f64(), rhs.as_f64()) {
                    self.set(name, StoryValue::Float(a - b));
                }
            }
        }
    }

    /// Interpolate `{var}` placeholders in text.
    pub fn interpolate(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                let mut name = String::new();
                let mut closed = false;
                for d in chars.by_ref() {
                    if d == '}' {
                        closed = true;
                        break;
                    }
                    name.push(d);
                }
                if closed {
                    let key = name.trim();
                    // Keep rich-text / style tags intact for the product Say path
                    // (`{cps=25}`, `{color=#ff0}`, `{/color}`, …). Only bare
                    // identifiers are treated as story variables.
                    let is_markup = key.is_empty()
                        || key.contains('=')
                        || key.starts_with('/')
                        || key.contains('#');
                    if is_markup {
                        out.push('{');
                        out.push_str(&name);
                        out.push('}');
                    } else {
                        out.push_str(&self.get(key).display_str());
                    }
                } else {
                    out.push('{');
                    out.push_str(&name);
                }
            } else {
                out.push(c);
            }
        }
        out
    }
}

/// Assignment operator for story scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignOp {
    /// `=`
    Set,
    /// `+=`
    Add,
    /// `-=`
    Sub,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_vars() {
        let mut v = StoryVariables::new();
        v.set("player", StoryValue::String("Alex".into()));
        v.set("n", StoryValue::Int(3));
        assert_eq!(v.interpolate("Hi {player}, score {n}"), "Hi Alex, score 3");
    }

    #[test]
    fn add_assign() {
        let mut v = StoryVariables::new();
        v.set("trust", StoryValue::Int(1));
        v.apply_assign("trust", AssignOp::Add, StoryValue::Int(2));
        assert_eq!(v.get_int("trust", 0), 3);
    }
}
