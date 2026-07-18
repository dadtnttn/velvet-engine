//! Inline icon placeholder spans `{icon=name}`.

use std::collections::HashMap;

/// Parsed inline icon placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct IconSpan {
    /// Icon registry key / name.
    pub name: String,
    /// Optional size override in pixels.
    pub size: Option<f32>,
}

impl IconSpan {
    /// Create named icon.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: None,
        }
    }

    /// With size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size.max(1.0));
        self
    }
}

/// Parse `{icon=name}` / `{icon id=name}` / `{icon name size=16}` attribute body.
pub fn parse_icon_attrs(attrs: &str) -> IconSpan {
    let mut name = None;
    let mut size = None;
    if !attrs.contains('=') && !attrs.is_empty() {
        name = Some(attrs.trim_matches('"').to_string());
    } else {
        for part in attrs.split_whitespace() {
            if let Some((k, v)) = part.split_once('=') {
                match k {
                    "id" | "name" | "value" => name = Some(v.trim_matches('"').to_string()),
                    "size" => size = v.parse().ok(),
                    _ => {}
                }
            }
        }
        // form icon=sword from split_tag when attrs is "sword"
        if name.is_none() && attrs.contains('=') {
            if let Some((k, v)) = attrs.split_once('=') {
                if k == "icon" || k.is_empty() {
                    name = Some(v.to_string());
                }
            }
        }
    }
    let mut span = IconSpan::new(name.unwrap_or_else(|| "icon".into()));
    if let Some(s) = size {
        span = span.with_size(s);
    }
    span
}

/// Format icon markup.
pub fn format_icon_markup(name: &str) -> String {
    format!("{{icon={name}}}")
}

/// Simple name → glyph fallback map for CPU layout without texture atlases.
#[derive(Debug, Clone, Default)]
pub struct IconGlyphMap {
    map: HashMap<String, char>,
}

impl IconGlyphMap {
    /// Empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Built-in defaults.
    pub fn with_defaults() -> Self {
        let mut m = Self::new();
        m.insert("heart", '♥');
        m.insert("star", '★');
        m.insert("coin", '◎');
        m.insert("key", '⚷');
        m.insert("sword", '⚔');
        m.insert("warning", '⚠');
        m.insert("check", '✓');
        m
    }

    /// Insert mapping.
    pub fn insert(&mut self, name: impl Into<String>, glyph: char) {
        self.map.insert(name.into(), glyph);
    }

    /// Lookup glyph; default `◆`.
    pub fn glyph(&self, name: &str) -> char {
        self.map.get(name).copied().unwrap_or('◆')
    }

    /// Replace `{icon=…}` placeholders in plain text with glyphs (no nested tags).
    pub fn expand_plain(&self, input: &str) -> String {
        let mut out = String::new();
        let mut rest = input;
        while let Some(start) = rest.find("{icon") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            if let Some(end) = after.find('}') {
                let body = &after[..end];
                // body like "icon=sword" or "icon id=sword"
                let attrs = body
                    .strip_prefix("icon")
                    .unwrap_or(body)
                    .trim_start_matches(|c: char| c == '=' || c.is_whitespace());
                let span = parse_icon_attrs(attrs);
                out.push(self.glyph(&span.name));
                rest = &after[end + 1..];
            } else {
                out.push_str(rest);
                return out;
            }
        }
        out.push_str(rest);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_icon_eq_form() {
        let s = parse_icon_attrs("sword");
        assert_eq!(s.name, "sword");
    }

    #[test]
    fn parse_icon_id() {
        let s = parse_icon_attrs("id=heart size=18");
        assert_eq!(s.name, "heart");
        assert_eq!(s.size, Some(18.0));
    }

    #[test]
    fn expand_plain() {
        let map = IconGlyphMap::with_defaults();
        let t = map.expand_plain("Get {icon=coin} x3");
        assert!(t.contains('◎'));
        assert!(t.contains("x3"));
    }

    #[test]
    fn format_markup() {
        assert_eq!(format_icon_markup("star"), "{icon=star}");
    }
}
