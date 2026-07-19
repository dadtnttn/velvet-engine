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
    if let Some(c) = parse_color(s) {
        return StyleValue::Color(c);
    }
    if let Some(num) = s.strip_suffix("px").or(Some(s)) {
        if let Ok(v) = num.trim().parse::<f32>() {
            if s.ends_with("px") || !s.contains(char::is_alphabetic) {
                return if s.ends_with("px") {
                    StyleValue::Length(v)
                } else if s.contains('.') || s.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '.')
                {
                    // bare number: length if looks like size context — keep as Number
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
