//! Colours: straight (not premultiplied) RGBA, eight bits a channel.

/// A colour in straight RGBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    /// An opaque colour from `0xRRGGBB`.
    pub const fn hex(rgb: u32) -> Self {
        Self::rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
    }

    /// A colour from `0xRRGGBBAA`.
    pub const fn hexa(rgba: u32) -> Self {
        Self::rgba(
            (rgba >> 24) as u8,
            (rgba >> 16) as u8,
            (rgba >> 8) as u8,
            rgba as u8,
        )
    }

    /// The same colour with its alpha multiplied by `factor` (0 to 1).
    pub fn fade(self, factor: f32) -> Self {
        Self {
            a: (self.a as f32 * factor.clamp(0.0, 1.0)).round() as u8,
            ..self
        }
    }

    /// The colour mixed towards `other` by `t` (0 gives this colour, 1 gives `other`).
    pub fn mix(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Self::rgba(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
            lerp(self.a, other.a),
        )
    }

    pub(crate) fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_rgb_and_rgba() {
        assert_eq!(Color::hex(0x112233), Color::rgba(0x11, 0x22, 0x33, 255));
        assert_eq!(Color::hexa(0x11223344), Color::rgba(0x11, 0x22, 0x33, 0x44));
    }

    #[test]
    fn mix_and_fade() {
        assert_eq!(
            Color::BLACK.mix(Color::WHITE, 0.5),
            Color::rgba(128, 128, 128, 255)
        );
        assert_eq!(Color::WHITE.fade(0.5).a, 128);
    }
}
