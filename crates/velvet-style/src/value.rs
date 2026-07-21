//! Style values (colors, lengths, keywords).

use serde::{Deserialize, Serialize};

/// Parsed color (sRGB 0–255 + alpha 0–1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha 0..=1.
    pub a: f32,
}

impl Color {
    /// RGB opaque.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// RGBA.
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Pack to 0x00RRGGBB (softbuffer-friendly).
    pub fn pack_rgb(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Tuple for demo paint helpers.
    pub fn rgb_tuple(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }
}

/// One CSS-like value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StyleValue {
    /// Color.
    Color(Color),
    /// Length in logical px (unitless number treated as px).
    Length(f32),
    /// Number (opacity, scale, …).
    Number(f32),
    /// Time in seconds (`s` and `ms` inputs normalize to this variant).
    Time(f32),
    /// Keyword / string token.
    Keyword(String),
    /// String (quoted).
    String(String),
    /// Custom property reference: `var(--name)`.
    Var(String),
    /// Custom property reference with a fallback: `var(--name, fallback)`.
    ///
    /// The existing [`StyleValue::Var`] variant remains the representation for
    /// references without a fallback.
    VarFallback {
        /// Canonical custom property name, including the `--` prefix.
        name: String,
        /// Value used when the custom property is missing or cyclic.
        fallback: Box<StyleValue>,
    },
    /// Asset / resource URL: `url("path")` or `svg(name)`.
    Url(String),
    /// Named inline SVG from `@svg name { … }`.
    SvgRef(String),
}

impl StyleValue {
    /// As color if possible.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            Self::Color(c) => Some(*c),
            Self::Keyword(k) => parse_named_color(k),
            _ => None,
        }
    }

    /// As f32 (length or number).
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Length(v) | Self::Number(v) | Self::Time(v) => Some(*v),
            _ => None,
        }
    }

    /// As string keyword.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Keyword(s) | Self::String(s) => Some(s),
            Self::Var(s) | Self::VarFallback { name: s, .. } => Some(s.as_str()),
            _ => None,
        }
    }

    /// Custom property name if this is a `var(--…)`.
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Self::Var(s) | Self::VarFallback { name: s, .. } => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Parse `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, or named.
pub fn parse_color(raw: &str) -> Option<Color> {
    let s = raw.trim();
    if let Some(c) = parse_named_color(s) {
        return Some(c);
    }
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(inner) = s
        .strip_prefix("rgba(")
        .or_else(|| s.strip_prefix("rgb("))
        .and_then(|t| t.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            let a = if parts.len() >= 4 {
                parts[3].parse().unwrap_or(1.0)
            } else {
                1.0
            };
            return Some(Color::rgba(r, g, b, a));
        }
    }
    None
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim();
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_named_color(name: &str) -> Option<Color> {
    Some(match name.to_ascii_lowercase().as_str() {
        "transparent" => Color::rgba(0, 0, 0, 0.0),
        "black" => Color::rgb(0, 0, 0),
        "white" => Color::rgb(255, 255, 255),
        "gold" => Color::rgb(235, 200, 120),
        "purple" => Color::rgb(120, 60, 180),
        "magenta" => Color::rgb(220, 80, 220),
        "navy" => Color::rgb(10, 12, 22),
        "neon" => Color::rgb(170, 100, 255),
        _ => return None,
    })
}

/// Parse a single property value token string.
pub fn parse_value(raw: &str) -> StyleValue {
    let s = raw.trim().trim_end_matches(';').trim();
    // var(--token) or var(--token, fallback)
    if let Some(inner) = s
        .strip_prefix("var(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let (name, fallback) = split_top_level_comma(inner);
        let name = name.trim();
        let name = name.trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            let canonical = if name.starts_with("--") {
                name.to_string()
            } else {
                format!("--{name}")
            };
            if let Some(fallback) = fallback.filter(|value| !value.trim().is_empty()) {
                return StyleValue::VarFallback {
                    name: canonical,
                    fallback: Box::new(parse_value(fallback)),
                };
            }
            return StyleValue::Var(canonical);
        }
    }
    // url("path") / url('path')
    if let Some(inner) = s
        .strip_prefix("url(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let path = inner.trim_matches(|c| c == '"' || c == '\'');
        if !path.is_empty() {
            return StyleValue::Url(path.to_string());
        }
    }
    // svg(name)
    if let Some(inner) = s
        .strip_prefix("svg(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let name = inner.trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            return StyleValue::SvgRef(name.to_string());
        }
    }
    if let Some(c) = parse_color(s) {
        return StyleValue::Color(c);
    }
    if has_time_suffix(s) {
        if let Some(seconds) = parse_time_seconds_token(s) {
            return StyleValue::Time(seconds);
        }
    }
    if let Some(num) = s.strip_suffix("px").or(Some(s)) {
        if let Ok(v) = num.trim().parse::<f32>() {
            if s.ends_with("px") || !s.contains(char::is_alphabetic) {
                return if s.ends_with("px") {
                    StyleValue::Length(v)
                } else if s.contains('.')
                    || s.chars()
                        .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
                {
                    StyleValue::Number(v)
                } else {
                    StyleValue::Number(v)
                };
            }
        }
    }
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return StyleValue::String(s[1..s.len() - 1].to_string());
    }
    StyleValue::Keyword(s.to_string())
}

fn has_time_suffix(raw: &str) -> bool {
    raw.strip_suffix("ms").is_some() || raw.strip_suffix('s').is_some()
}

/// Parse a time token into seconds.
///
/// Unitless tokens retain the historical shorthand behavior and are interpreted
/// as seconds. Full declaration values only become [`StyleValue::Time`] when a
/// `s` or `ms` suffix is present.
pub(crate) fn parse_time_seconds_token(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    if let Some(ms) = raw.strip_suffix("ms") {
        return ms.trim().parse::<f32>().ok().map(|value| value / 1000.0);
    }
    if let Some(seconds) = raw.strip_suffix('s') {
        return seconds.trim().parse::<f32>().ok();
    }
    raw.parse::<f32>().ok()
}

fn split_top_level_comma(input: &str) -> (&str, Option<&str>) {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0u32;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(end_quote) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == end_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                return (&input[..index], Some(&input[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    (input, None)
}

/// Known game-UI property names (documentation / tooling).
pub const KNOWN_PROPERTIES: &[&str] = &[
    // colors / chrome
    "background",
    "background-image",
    "background-size",
    "color",
    "border-color",
    "border-width",
    "border-radius",
    "border-style",
    "glow",
    "glow-strength",
    "box-shadow",
    "opacity",
    // box
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "margin",
    "margin-x",
    "margin-y",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-x",
    "padding-y",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "gap",
    // type
    "font-size",
    "font-family",
    "font-weight",
    "letter-spacing",
    "text-align",
    "line-height",
    // transform / motion static
    "x",
    "y",
    "scale",
    "rotate",
    "yaw",
    "pitch",
    "roll",
    "translate-x",
    "translate-y",
    // animation
    "animation",
    "animation-name",
    "animation-duration",
    "animation-delay",
    "animation-timing-function",
    "animation-ease",
    "animation-iteration-count",
    "animation-fill-mode",
    "animation-target",
    "transition",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    // game chrome
    "icon",
    "icon-size",
    "gold",
    "neon",
    "foil",
    "depth",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_seconds_and_milliseconds() {
        assert_eq!(parse_value("0.25s"), StyleValue::Time(0.25));
        assert_eq!(parse_value("180ms"), StyleValue::Time(0.18));
        assert_eq!(parse_value("12"), StyleValue::Number(12.0));
    }

    #[test]
    fn parses_var_fallback_without_changing_plain_var() {
        assert_eq!(parse_value("var(--gold)"), StyleValue::Var("--gold".into()));
        assert_eq!(
            parse_value("var(--gold, var(--accent, #ffffff))"),
            StyleValue::VarFallback {
                name: "--gold".into(),
                fallback: Box::new(StyleValue::VarFallback {
                    name: "--accent".into(),
                    fallback: Box::new(StyleValue::Color(Color::rgb(255, 255, 255))),
                }),
            }
        );
    }
}
