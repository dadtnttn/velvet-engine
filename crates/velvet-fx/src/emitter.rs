//! Emitter shape + rate.

use serde::{Deserialize, Serialize};

/// Overall emitter shape (spawn distribution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmitterShape {
    /// Point burst at path sample.
    #[default]
    Point,
    /// Spray cone outward from path normal.
    Cone,
    /// Full circle burst.
    Circle,
    /// Directed (e.g. upward flame).
    Directed,
    /// Ring outward.
    Ring,
}

impl EmitterShape {
    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Point => "Point",
            Self::Cone => "Cone",
            Self::Circle => "Circle",
            Self::Directed => "Directed",
            Self::Ring => "Ring",
        }
    }
}

/// Emitter config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitterConfig {
    /// Shape.
    pub shape: EmitterShape,
    /// Particles per second (or per frame budget)
    pub rate: f32,
    /// Burst count (when using burst rather than continuous)
    pub burst_count: u32,
    /// Continuous emission (true) vs one-shot burst per loop
    pub continuous: bool,
    /// Spread angle radians (half-angle for cone)
    pub spread: f32,
    /// Base angle offset radians
    pub angle: f32,
    /// Inherit path normal (true = add path normal to angle)
    pub align_to_path: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            shape: EmitterShape::Cone,
            rate: 90.0,
            burst_count: 24,
            continuous: true,
            spread: 0.7,
            angle: -std::f32::consts::FRAC_PI_2,
            align_to_path: true,
        }
    }
}
