//! Simple material parameters for 2D.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

/// Blend modes supported by the 2D pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendMode {
    /// Standard alpha blending.
    #[default]
    Alpha,
    /// Additive (lights, particles).
    Additive,
    /// Multiply.
    Multiply,
    /// Opaque (no blending).
    Opaque,
}

/// Material-like parameters (expandable for custom shaders later).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// Base tint.
    pub tint: Color,
    /// Blend mode.
    pub blend: BlendMode,
    /// Optional shader key / name for future custom pipelines.
    pub shader: Option<String>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            tint: Color::WHITE,
            blend: BlendMode::Alpha,
            shader: None,
        }
    }
}
