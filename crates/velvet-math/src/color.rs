//! Color types (linear float and 8-bit sRGB-ish storage).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{clamp, lerp, Vec3};

/// Linear RGBA color with `f32` components in roughly `[0, 1]` (not enforced).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Color {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Color {
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    /// Red.
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Green.
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    /// Blue.
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    /// Magenta-ish brand accent placeholder.
    pub const VELVET: Self = Self {
        r: 1.0,
        g: 0.31,
        b: 0.545,
        a: 1.0,
    };

    /// Create RGBA.
    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create opaque RGB.
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// From hex `0xRRGGBB` or `0xRRGGBBAA` (alpha defaults to FF if 24-bit).
    pub fn from_hex(hex: u32) -> Self {
        if hex > 0x00FF_FFFF {
            let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
            let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
            let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
            let a = (hex & 0xFF) as f32 / 255.0;
            Self { r, g, b, a }
        } else {
            let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
            let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
            let b = (hex & 0xFF) as f32 / 255.0;
            Self { r, g, b, a: 1.0 }
        }
    }

    /// Parse `#RGB`, `#RRGGBB`, `#RRGGBBAA` (optional leading `#`).
    pub fn parse_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
                Some(Self::from_color8(Color8 { r, g, b, a: 255 }))
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::from_color8(Color8 { r, g, b, a: 255 }))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::from_color8(Color8 { r, g, b, a }))
            }
            _ => None,
        }
    }

    /// From 8-bit color.
    pub fn from_color8(c: Color8) -> Self {
        Self {
            r: c.r as f32 / 255.0,
            g: c.g as f32 / 255.0,
            b: c.b as f32 / 255.0,
            a: c.a as f32 / 255.0,
        }
    }

    /// To 8-bit color (clamped).
    pub fn to_color8(self) -> Color8 {
        Color8 {
            r: (clamp(self.r, 0.0, 1.0) * 255.0).round() as u8,
            g: (clamp(self.g, 0.0, 1.0) * 255.0).round() as u8,
            b: (clamp(self.b, 0.0, 1.0) * 255.0).round() as u8,
            a: (clamp(self.a, 0.0, 1.0) * 255.0).round() as u8,
        }
    }

    /// RGB as [`Vec3`].
    pub const fn rgb_vec3(self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }

    /// Multiply RGB by alpha (premultiply).
    pub fn premultiply(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    /// Linear interpolation.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: lerp(self.r, other.r, t),
            g: lerp(self.g, other.g, t),
            b: lerp(self.b, other.b, t),
            a: lerp(self.a, other.a, t),
        }
    }

    /// Multiply RGB by a scalar tint, keep alpha.
    pub fn tint(self, factor: f32) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a,
        }
    }

    /// With replaced alpha.
    pub const fn with_alpha(self, a: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    /// To `[r,g,b,a]` array.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// 8-bit RGBA color (storage / GPU textures).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Color8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha.
    pub a: u8,
}

impl Color8 {
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Create RGBA.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Pack to `0xAARRGGBB` (common GPU order varies; document as AARRGGBB).
    pub const fn to_u32_aarrggbb(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_colors() {
        let c = Color::parse_hex("#ff4f8b").unwrap();
        assert!((c.r - 1.0).abs() < 1e-3);
        assert!((c.a - 1.0).abs() < 1e-5);
        let short = Color::parse_hex("f0a").unwrap();
        assert!(short.r > 0.9);
    }

    #[test]
    fn hex_u32() {
        let c = Color::from_hex(0x00FF_0000);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!(c.g.abs() < 1e-5);
    }
}
