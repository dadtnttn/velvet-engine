//! Pro dialogue text: b/i/color + ruby, CJK/emoji width, voice-timed speed.
//!
//! The product say path strips markup for display; this module keeps the
//! structure so hosts can render bold/italic/color, furigana, and compute
//! typewriter + voice timing on **visible** characters only.

use serde::{Deserialize, Serialize};

/// Inline style flags for a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextStyleFlags {
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
}

/// One visible span with optional color + ruby (furigana).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RichSpan {
    /// Visible text (no tags).
    pub text: String,
    /// Style flags.
    pub style: TextStyleFlags,
    /// Color `#rrggbb` or named, without braces.
    #[serde(default)]
    pub color: Option<String>,
    /// Ruby/furigana reading shown above this span.
    #[serde(default)]
    pub ruby: Option<String>,
}

impl RichSpan {
    /// Plain span.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyleFlags::default(),
            color: None,
            ruby: None,
        }
    }
}

/// Parsed dialogue line.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RichLine {
    /// Spans in order.
    pub spans: Vec<RichSpan>,
    /// Optional `{cps=N}` override.
    pub cps: Option<f32>,
}

impl RichLine {
    /// Plain text (spans joined, ruby excluded).
    pub fn plain(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Visible character count (tags excluded, ruby excluded).
    pub fn visible_len(&self) -> usize {
        self.spans.iter().map(|s| s.text.chars().count()).sum()
    }

    /// Display width: CJK/emoji count 2, others 1. Used for say-box measure.
    pub fn display_width(&self) -> usize {
        self.spans
            .iter()
            .flat_map(|s| s.text.chars())
            .map(char_width)
            .sum()
    }

    /// Has any styling.
    pub fn has_style(&self) -> bool {
        self.spans
            .iter()
            .any(|s| s.style.bold || s.style.italic || s.color.is_some() || s.ruby.is_some())
    }
}

/// Width of one char for VN measure: 2 for CJK/emoji, else 1.
pub fn char_width(ch: char) -> usize {
    let u = ch as u32;
    if (0x4E00..=0x9FFF).contains(&u)   // CJK unified
        || (0x3040..=0x30FF).contains(&u) // hiragana/katakana
        || (0xAC00..=0xD7AF).contains(&u) // hangul
        || (0x3400..=0x4DBF).contains(&u)
        || (0x1F300..=0x1FAFF).contains(&u) // emoji blocks
        || (0x2600..=0x27BF).contains(&u)
        || (0xFE00..=0xFE0F).contains(&u)
    {
        2
    } else {
        1
    }
}

/// Parse `{b}{i}{color=..}{ruby=..}{cps=N}` markup into a [`RichLine`].
///
/// Unknown `{...}` tags are stripped (forward-compat). `{/b}{/i}{/color}{/ruby}`
/// close the matching style. Unclosed spans run to end of line.
pub fn parse_rich_line(markup: &str) -> RichLine {
    let mut line = RichLine::default();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut color: Option<String> = None;
    let mut ruby: Option<String> = None;

    let flush = |buf: &mut String,
                 out: &mut Vec<RichSpan>,
                 bold: bool,
                 italic: bool,
                 color: &Option<String>,
                 ruby: &Option<String>| {
        if buf.is_empty() {
            return;
        }
        out.push(RichSpan {
            text: std::mem::take(buf),
            style: TextStyleFlags { bold, italic },
            color: color.clone(),
            ruby: ruby.clone(),
        });
    };

    let chars: Vec<char> = markup.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            if let Some(rel) = chars[i..].iter().position(|&c| c == '}') {
                let tag: String = chars[i + 1..i + rel].iter().collect();
                let t = tag.trim();
                // flush before style change
                flush(&mut buf, &mut line.spans, bold, italic, &color, &ruby);
                if t == "b" {
                    bold = true;
                } else if t == "/b" {
                    bold = false;
                } else if t == "i" {
                    italic = true;
                } else if t == "/i" {
                    italic = false;
                } else if let Some(rest) = t.strip_prefix("color=") {
                    let c = rest.trim();
                    color = if c.is_empty() {
                        None
                    } else {
                        Some(c.to_string())
                    };
                } else if t == "/color" {
                    color = None;
                } else if let Some(rest) = t.strip_prefix("ruby=") {
                    let r = rest.trim();
                    ruby = if r.is_empty() {
                        None
                    } else {
                        Some(r.to_string())
                    };
                } else if t == "/ruby" {
                    ruby = None;
                } else if let Some(rest) = t.strip_prefix("cps=") {
                    if let Ok(v) = rest.parse::<f32>() {
                        line.cps = Some(v);
                    }
                }
                // {w}, {w=..}, {size=..}, {pause=..}, {shake..} -> stripped
                i += rel + 1;
                continue;
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush(&mut buf, &mut line.spans, bold, italic, &color, &ruby);
    line
}

/// Estimate voice seconds for a line: visible chars / effective cps.
///
/// CJK reads slower (0.8x chars/sec factor via width), voice pitch shifts
/// duration slightly. Used when clip duration is unknown so auto-advance
/// still waits a fair time.
pub fn estimate_voice_secs(plain_or_markup: &str, cps: f32, voice_cps_scale: f32) -> f32 {
    let line = parse_rich_line(plain_or_markup);
    let n = line.visible_len().max(1) as f32;
    let width_factor = if line.display_width() > line.visible_len() {
        1.25 // CJK/emoji heavy: a bit longer
    } else {
        1.0
    };
    let eff_cps = (cps * voice_cps_scale.max(0.2)).max(1.0);
    (n / eff_cps) * width_factor + 0.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_styles_and_ruby() {
        let l = parse_rich_line("Hi {b}bold{/b} {color=#f00}red{ruby=aka}X{/ruby}{/color}!");
        assert!(l.has_style());
        assert_eq!(l.plain(), "Hi bold redX!");
        assert_eq!(l.visible_len(), "Hi bold redX!".chars().count());
        let red = l.spans.iter().find(|s| s.text == "red").unwrap();
        assert_eq!(red.color.as_deref(), Some("#f00"));
        let x = l.spans.iter().find(|s| s.text == "X").unwrap();
        assert_eq!(x.ruby.as_deref(), Some("aka"));
    }

    #[test]
    fn cps_and_strip_unknown() {
        let l = parse_rich_line("{cps=30}Hello {w=0.5}world{shake}!");
        assert_eq!(l.cps, Some(30.0));
        assert_eq!(l.plain(), "Hello world!");
    }

    #[test]
    fn width_cjk_emoji_double() {
        assert_eq!(char_width('A'), 1);
        assert_eq!(char_width('あ'), 2);
        assert_eq!(char_width('中'), 2);
        let l = parse_rich_line("Hi中😀");
        assert_eq!(l.visible_len(), 4);
        assert_eq!(l.display_width(), 1 + 1 + 2 + 2);
    }

    #[test]
    fn voice_estimate_scales() {
        let short = estimate_voice_secs("Hi", 20.0, 1.0);
        let long = estimate_voice_secs("Hello world, this is longer", 20.0, 1.0);
        assert!(long > short);
        let slow_voice = estimate_voice_secs("Hello", 20.0, 0.5);
        let fast_voice = estimate_voice_secs("Hello", 20.0, 2.0);
        assert!(slow_voice > fast_voice);
    }
}
