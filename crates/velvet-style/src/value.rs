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
    /// Keyword / string token.
    Keyword(String),
    /// String (quoted).
    String(String),
    /// Custom property reference: `var(--name)`.
    Var(String),
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
            Self::Length(v) | Self::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// As string keyword.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Keyword(s) | Self::String(s) => Some(s),
            Self::Var(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Custom property name if this is a `var(--…)`.
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Self::Var(s) => Some(s.as_str()),
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
    // var(--token) or var(--token, fallback) — fallback parsed if present
    if let Some(inner) = s
        .strip_prefix("var(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let name = inner.split(',').next().unwrap_or(inner).trim();
        let name = name.trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            let canonical = if name.starts_with("--") {
                name.to_string()
            } else {
                format!("--{name}")
            };
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
