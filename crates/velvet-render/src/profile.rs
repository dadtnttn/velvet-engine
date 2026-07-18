//! Named render quality profiles.

use serde::{Deserialize, Serialize};

use crate::letterbox::ScalingMode;

/// Preset rendering profile for a genre or look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RenderProfile {
    /// Soft UI-oriented presentation for visual novels.
    VisualNovel,
    /// Nearest filtering, integer scale.
    PixelArt,
    /// Balanced defaults for exploration RPGs.
    TopDownRpg,
    /// Prefer performance for dense action scenes.
    TopDownAction,
    /// Higher resolution targets / softer filters for cinematics.
    Cinematic2d,
    /// Custom / engine default.
    #[default]
    Default,
}

impl RenderProfile {
    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::VisualNovel => "Visual Novel",
            Self::PixelArt => "Pixel Art",
            Self::TopDownRpg => "Top-down RPG",
            Self::TopDownAction => "Top-down Action",
            Self::Cinematic2d => "Cinematic 2D",
            Self::Default => "Default",
        }
    }

    /// Preferred scaling mode.
    pub fn scaling_mode(self) -> ScalingMode {
        match self {
            Self::PixelArt => ScalingMode::IntegerScale,
            Self::Cinematic2d | Self::VisualNovel => ScalingMode::Letterbox,
            Self::TopDownAction => ScalingMode::Letterbox,
            Self::TopDownRpg | Self::Default => ScalingMode::Letterbox,
        }
    }

    /// Whether textures should prefer nearest sampling.
    pub fn nearest_textures(self) -> bool {
        matches!(self, Self::PixelArt | Self::TopDownAction)
    }

    /// Suggested max sprites before warning (soft budget).
    pub fn sprite_soft_budget(self) -> u32 {
        match self {
            Self::VisualNovel => 256,
            Self::PixelArt => 2_000,
            Self::TopDownRpg => 4_000,
            Self::TopDownAction => 8_000,
            Self::Cinematic2d => 1_000,
            Self::Default => 4_000,
        }
    }

    /// MSAA sample count preference (1 = none). Actual device may clamp.
    pub fn msaa_samples(self) -> u32 {
        match self {
            Self::Cinematic2d | Self::VisualNovel => 4,
            Self::PixelArt => 1,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_art_uses_integer_scale() {
        assert_eq!(
            RenderProfile::PixelArt.scaling_mode(),
            ScalingMode::IntegerScale
        );
        assert!(RenderProfile::PixelArt.nearest_textures());
    }
}
