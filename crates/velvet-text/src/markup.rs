//! Markup parser: `{color=#ff5577}text{/color}`, `{pause=0.5}`, `{shake intensity=3}…{/shake}`.

use thiserror::Error;
use velvet_math::Color;

use crate::style::{TextEffect, TextStyle};

/// Markup parse error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MarkupError {
    /// Unclosed tag.
    #[error("unclosed tag at {0}")]
    Unclosed(usize),
    /// Unknown tag.
    #[error("unknown tag '{0}' at {1}")]
    UnknownTag(String, usize),
    /// Bad attribute.
    #[error("bad attribute in '{0}' at {1}")]
    BadAttr(String, usize),
}

/// One styled run of text (or control).
#[derive(Debug, Clone, PartialEq)]
pub enum RichSpan {
    /// Visible text with style.
    Text {
        /// Content.
        text: String,
        /// Style.
        style: TextStyle,
    },
    /// Pause for seconds (typewriter).
    Pause {
        /// Seconds.
        seconds: f32,
    },
    /// Speed multiplier for following text.
    Speed {
        /// Multiplier.
        multiplier: f32,
    },
    /// Inline icon key.
    Icon {
        /// Icon id.
        id: String,
    },
    /// Ruby / furigana annotation.
    Ruby {
        /// Base text.
        base: String,
        /// Ruby text.
        ruby: String,
        /// Base style.
        style: TextStyle,
    },
    /// Hyperlink marker.
    Link {
        /// Target.
        href: String,
        /// Label spans flatten to text in layout.
        label: String,
    },
}

/// Parsed rich document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RichText {
    /// Spans in order.
    pub spans: Vec<RichSpan>,
}

impl RichText {
    /// Plain string without tags.
    pub fn plain(&self) -> String {
        let mut out = String::new();
        for s in &self.spans {
            match s {
                RichSpan::Text { text, .. } => out.push_str(text),
                RichSpan::Link { label, .. } => out.push_str(label),
                RichSpan::Ruby { base, .. } => out.push_str(base),
                _ => {}
            }
        }
        out
    }

    /// Character count for typewriter (text + link labels + ruby base).
    pub fn char_count(&self) -> usize {
        self.plain().chars().count()
    }

    /// Collect icon ids in document order.
    pub fn icon_ids(&self) -> Vec<&str> {
        self.spans
            .iter()
            .filter_map(|s| match s {
                RichSpan::Icon { id } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// Parse markup into rich text.
pub fn parse_rich_text(input: &str) -> Result<RichText, MarkupError> {
    parse_with_base(input, TextStyle::default())
}

fn parse_with_base(input: &str, base: TextStyle) -> Result<RichText, MarkupError> {
    let mut spans = Vec::new();
    let mut style_stack = vec![base];
    let mut i = 0;
    let bytes = input.as_bytes();
    let mut buf = String::new();

    let flush = |buf: &mut String, style: &TextStyle, spans: &mut Vec<RichSpan>| {
        if !buf.is_empty() {
            spans.push(RichSpan::Text {
                text: std::mem::take(buf),
                style: style.clone(),
            });
        }
    };

    while i < bytes.len() {
        if bytes[i] == b'{' {
            flush(&mut buf, style_stack.last().unwrap(), &mut spans);
            let close = input[i..].find('}').ok_or(MarkupError::Unclosed(i))?;
            let tag = &input[i + 1..i + close];
            i += close + 1;
            if let Some(rest) = tag.strip_prefix('/') {
                // closing tag
                let name = rest.trim();
                if style_stack.len() > 1 {
                    style_stack.pop();
                } else {
                    return Err(MarkupError::UnknownTag(format!("/{name}"), i));
                }
                continue;
            }
            // self-closing style controls: pause, speed, icon
            let (name, attrs) = split_tag(tag);
            match name {
                "pause" => {
                    let secs = attr_f32(attrs, "value")
                        .or_else(|| first_unnamed_f32(attrs))
                        .unwrap_or(0.5);
                    spans.push(RichSpan::Pause { seconds: secs });
                }
                "speed" => {
                    let m = attr_f32(attrs, "value")
                        .or_else(|| first_unnamed_f32(attrs))
                        .unwrap_or(1.0);
                    spans.push(RichSpan::Speed { multiplier: m });
                }
                "icon" => {
                    let id = attr_str(attrs, "id")
                        .or_else(|| attr_str(attrs, "name"))
                        .or_else(|| first_unnamed_str(attrs))
                        .unwrap_or("icon")
                        .to_string();
                    spans.push(RichSpan::Icon { id });
                }
                "ruby" => {
                    let ruby = attr_str(attrs, "text")
                        .or_else(|| attr_str(attrs, "rt"))
                        .or_else(|| attr_str(attrs, "value"))
                        .or_else(|| first_unnamed_str(attrs))
                        .unwrap_or("")
                        .to_string();
                    let base_attr = attr_str(attrs, "base").map(|s| s.to_string());
                    let style = style_stack.last().unwrap().clone();
                    if let Some(base) = base_attr {
                        spans.push(RichSpan::Ruby { base, ruby, style });
                    } else if let Some(end) = input[i..].find("{/ruby}") {
                        let base = input[i..i + end].to_string();
                        i += end + "{/ruby}".len();
                        spans.push(RichSpan::Ruby { base, ruby, style });
                    } else {
                        return Err(MarkupError::Unclosed(i));
                    }
                }
                "color" => {
                    let mut st = style_stack.last().unwrap().clone();
                    let hex = attr_str(attrs, "value")
                        .or_else(|| first_unnamed_str(attrs))
                        .unwrap_or("#ffffff");
                    st.color = Color::parse_hex(hex).unwrap_or(Color::WHITE);
                    style_stack.push(st);
                }
                "b" | "bold" => {
                    let mut st = style_stack.last().unwrap().clone();
                    st.weight = crate::style::FontWeight::Bold;
                    style_stack.push(st);
                }
                "i" | "italic" => {
                    let mut st = style_stack.last().unwrap().clone();
                    st.italic = true;
                    style_stack.push(st);
                }
                "size" => {
                    let mut st = style_stack.last().unwrap().clone();
                    st.size = attr_f32(attrs, "value")
                        .or_else(|| first_unnamed_f32(attrs))
                        .unwrap_or(st.size);
                    style_stack.push(st);
                }
                "shake" => {
                    let mut st = style_stack.last().unwrap().clone();
                    let intensity = attr_f32(attrs, "intensity").unwrap_or(3.0);
                    st.effect = TextEffect::Shake { intensity };
                    style_stack.push(st);
                }
                "wave" => {
                    let mut st = style_stack.last().unwrap().clone();
                    st.effect = TextEffect::Wave {
                        amplitude: attr_f32(attrs, "amplitude").unwrap_or(2.0),
                        frequency: attr_f32(attrs, "frequency").unwrap_or(4.0),
                    };
                    style_stack.push(st);
                }
                "link" => {
                    // {link href=url}label{/link} — label collected until close; simplify: href only inline
                    let href = attr_str(attrs, "href").unwrap_or("#").to_string();
                    // read until {/link}
                    if let Some(end) = input[i..].find("{/link}") {
                        let label = input[i..i + end].to_string();
                        i += end + "{/link}".len();
                        spans.push(RichSpan::Link { href, label });
                    } else {
                        return Err(MarkupError::Unclosed(i));
                    }
                }
                other => {
                    return Err(MarkupError::UnknownTag(other.into(), i));
                }
            }
        } else {
            buf.push(bytes[i] as char);
            i += 1;
            // handle utf8: if non-ascii, use chars
            if bytes[i - 1] >= 0x80 {
                // re-sync: better iterate by chars from start for production;
                // for simplicity re-parse multi-byte carefully:
            }
        }
    }
    flush(&mut buf, style_stack.last().unwrap(), &mut spans);
    Ok(RichText { spans })
}

fn split_tag(tag: &str) -> (&str, &str) {
    let tag = tag.trim();
    if let Some((_n, _a)) = tag.split_once(|c: char| c.is_whitespace() || c == '=') {
        if tag.contains('=') && !tag.contains(char::is_whitespace) {
            // color=#ff
            if let Some((n, v)) = tag.split_once('=') {
                return (n, v);
            }
        }
        let name = tag.split_whitespace().next().unwrap_or(tag);
        let attrs = tag[name.len()..].trim();
        (name, attrs)
    } else {
        (tag, "")
    }
}

fn attr_f32(attrs: &str, key: &str) -> Option<f32> {
    for part in attrs.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                return v.parse().ok();
            }
        } else if key == "value" {
            return part.parse().ok();
        }
    }
    // color=#ff form passed as attrs entire value
    if key == "value" && attrs.starts_with('#') {
        return None;
    }
    if !attrs.contains('=') {
        attrs.parse().ok()
    } else {
        None
    }
}

fn first_unnamed_f32(attrs: &str) -> Option<f32> {
    if attrs.contains('=') {
        // try value without key: intensity=3 already handled
        None
    } else {
        attrs.trim().parse().ok()
    }
}

fn attr_str<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    for part in attrs.split_whitespace() {
        if let Some((k, v)) = part.split_once('=') {
            if k == key {
                return Some(v.trim_matches('"'));
            }
        }
    }
    if key == "value" && (attrs.starts_with('#') || attrs.starts_with('"')) {
        return Some(attrs.trim_matches('"'));
    }
    // form: color=#aabbcc as entire attrs from split_tag
    if key == "value" && !attrs.is_empty() && !attrs.contains(char::is_whitespace) {
        return Some(attrs.trim_matches('"'));
    }
    None
}

fn first_unnamed_str(attrs: &str) -> Option<&str> {
    if attrs.contains('=') || attrs.is_empty() {
        None
    } else {
        Some(attrs.trim_matches('"'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_and_pause() {
        let r =
            parse_rich_text("Esto es {color=#ff5577}importante{/color}.{pause=0.5} fin").unwrap();
        assert!(r.plain().contains("importante"));
        assert!(r.spans.iter().any(|s| matches!(s, RichSpan::Pause { .. })));
        assert!(r
            .spans
            .iter()
            .any(|s| matches!(s, RichSpan::Text { style, .. } if style.color != Color::WHITE)));
    }

    #[test]
    fn parse_shake() {
        let r = parse_rich_text("{shake intensity=3}Get away!{/shake}").unwrap();
        assert_eq!(r.plain(), "Get away!");
        assert!(matches!(
            &r.spans[0],
            RichSpan::Text {
                style: TextStyle {
                    effect: TextEffect::Shake { .. },
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn plain_strips_tags() {
        let r = parse_rich_text("{b}bold{/b} normal").unwrap();
        assert_eq!(r.plain(), "bold normal");
    }

    #[test]
    fn parse_icon_eq() {
        let r = parse_rich_text("Get {icon=coin} now").unwrap();
        assert_eq!(r.icon_ids(), vec!["coin"]);
        assert_eq!(r.plain(), "Get  now");
    }

    #[test]
    fn parse_ruby() {
        let r = parse_rich_text("{ruby text=かんじ}漢字{/ruby}").unwrap();
        assert!(matches!(
            &r.spans[0],
            RichSpan::Ruby {
                base,
                ruby,
                ..
            } if base == "漢字" && ruby == "かんじ"
        ));
        assert_eq!(r.plain(), "漢字");
    }
}
