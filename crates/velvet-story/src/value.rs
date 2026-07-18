//! Story variable values (stable for saves).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Serializable story value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum StoryValue {
    /// Null / unset.
    Null,
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    String(String),
}

impl StoryValue {
    /// Truthiness for conditions.
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Int(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.is_empty(),
        }
    }

    /// As i64 if numeric-ish.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Float(f) => Some(*f as i64),
            Self::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// As f64 if numeric-ish.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Interpolate into text: replace `{name}` later via variables.
    pub fn display_str(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Bool(b) => b.to_string(),
            Self::Int(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::String(s) => s.clone(),
        }
    }
}

impl fmt::Display for StoryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_str())
    }
}

impl From<bool> for StoryValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for StoryValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for StoryValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for StoryValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for StoryValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

/// Convert from script AST expression literals only (simple cases).
pub fn from_ast_expr(expr: &velvet_script_ast::Expr) -> Option<StoryValue> {
    use velvet_script_ast::Expr;
    match expr {
        Expr::Null { .. } => Some(StoryValue::Null),
        Expr::Bool { value, .. } => Some(StoryValue::Bool(*value)),
        Expr::Int { value, .. } => Some(StoryValue::Int(*value)),
        Expr::Float { value, .. } => Some(StoryValue::Float(*value)),
        Expr::String { value, .. } => Some(StoryValue::String(value.clone())),
        _ => None,
    }
}
