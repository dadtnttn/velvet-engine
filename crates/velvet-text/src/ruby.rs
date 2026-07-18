//! Ruby / furigana span support.

use crate::style::TextStyle;

/// A ruby (furigana) annotation over base text.
#[derive(Debug, Clone, PartialEq)]
pub struct RubySpan {
    /// Base text (kanji, etc.).
    pub base: String,
    /// Ruby text shown above/beside base.
    pub ruby: String,
    /// Style for base.
    pub base_style: TextStyle,
    /// Style for ruby (often smaller).
    pub ruby_style: TextStyle,
}

impl RubySpan {
    /// Create with default styles (ruby size = 50% of base).
    pub fn new(base: impl Into<String>, ruby: impl Into<String>) -> Self {
        let base_style = TextStyle::default();
        let mut ruby_style = base_style.clone();
        ruby_style.size = (base_style.size * 0.5).max(8.0);
        Self {
            base: base.into(),
            ruby: ruby.into(),
            base_style,
            ruby_style,
        }
    }

    /// With styles.
    pub fn with_styles(mut self, base: TextStyle, mut ruby: TextStyle) -> Self {
        if (ruby.size - TextStyle::default().size).abs() < 1e-3 {
            ruby.size = (base.size * 0.5).max(8.0);
        }
        self.base_style = base;
        self.ruby_style = ruby;
        self
    }

    /// Estimated layout width: max(base, ruby) using crude 0.5em per char.
    pub fn estimate_width(&self) -> f32 {
        let base_w = self.base.chars().count() as f32 * self.base_style.size * 0.5;
        let ruby_w = self.ruby.chars().count() as f32 * self.ruby_style.size * 0.5;
        base_w.max(ruby_w)
    }

    /// Estimated total height (base + ruby gap).
    pub fn estimate_height(&self) -> f32 {
        self.base_style.size * self.base_style.line_height + self.ruby_style.size * 0.9
    }
}

/// Parse `{ruby base=漢字 text=かんじ}` or `{ruby=かんじ}漢字{/ruby}` style tags from a fragment.
///
/// Returns `None` if the tag body is not a ruby tag.
pub fn parse_ruby_tag(tag_name: &str, attrs: &str) -> Option<(String, String)> {
    if tag_name != "ruby" {
        return None;
    }
    // form: ruby=かんじ later paired with content — attrs only
    let ruby = attr(attrs, "text")
        .or_else(|| attr(attrs, "rt"))
        .or_else(|| attr(attrs, "value"))
        .or_else(|| unnamed(attrs))?;
    let base = attr(attrs, "base").unwrap_or("").to_string();
    Some((base, ruby.to_string()))
}

fn attr<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    for part in attrs.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                return Some(v.trim_matches('"'));
            }
        }
    }
    // key=value only form without spaces when split_tag passes value as attrs
    if key == "value"
        && !attrs.is_empty()
        && !attrs.contains('=')
        && !attrs.contains(char::is_whitespace)
    {
        return Some(attrs.trim_matches('"'));
    }
    None
}

fn unnamed(attrs: &str) -> Option<&str> {
    if attrs.contains('=') || attrs.is_empty() {
        None
    } else {
        Some(attrs.trim_matches('"'))
    }
}

/// Format a ruby span as markup.
pub fn format_ruby_markup(base: &str, ruby: &str) -> String {
    format!("{{ruby text={ruby}}}{base}{{/ruby}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_attrs() {
        let (b, r) = parse_ruby_tag("ruby", "base=漢 text=かん").unwrap();
        assert_eq!(b, "漢");
        assert_eq!(r, "かん");
    }

    #[test]
    fn span_metrics() {
        let s = RubySpan::new("東京", "とうきょう");
        assert!(s.estimate_width() > 0.0);
        assert!(s.estimate_height() > s.base_style.size);
    }

    #[test]
    fn format_roundtrip_shape() {
        let m = format_ruby_markup("花", "はな");
        assert!(m.contains("花"));
        assert!(m.contains("はな"));
    }
}
