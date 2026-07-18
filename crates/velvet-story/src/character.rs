//! Character definitions.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

/// A named character in the story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Character {
    /// Stable id (script name).
    pub id: String,
    /// Display name (may be dynamic later).
    pub name: String,
    /// Name color.
    #[serde(default = "default_color")]
    pub color: String,
    /// Default portrait path.
    pub portrait: Option<String>,
    /// Expression → portrait path.
    #[serde(default)]
    pub expressions: indexmap::IndexMap<String, String>,
}

fn default_color() -> String {
    "#ffffff".into()
}

impl Character {
    /// Create minimal character.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: default_color(),
            portrait: None,
            expressions: indexmap::IndexMap::new(),
        }
    }

    /// Parse color hex if possible.
    pub fn color_value(&self) -> Color {
        Color::parse_hex(&self.color).unwrap_or(Color::WHITE)
    }

    /// Portrait for expression, fallback to default.
    pub fn portrait_for(&self, expression: Option<&str>) -> Option<&str> {
        if let Some(expr) = expression {
            if let Some(p) = self.expressions.get(expr) {
                return Some(p.as_str());
            }
            // id.expression style: "neutral" under expressions
            if let Some(p) = self.expressions.get(expr) {
                return Some(p.as_str());
            }
        }
        self.portrait.as_deref()
    }
}
