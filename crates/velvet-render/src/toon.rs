//! Shared art direction for the GPU world and lightweight CPU item previews.

/// Lighting response selected by a game; legacy engine clients stay lit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShadingStyle3D {
    /// Continuous diffuse/GGX response.
    #[default]
    Lit,
    /// Four broad diffuse levels, colored shadows and restrained cel highlights.
    Toon,
}

/// Compact toon controls also sent verbatim to the GPU camera uniform.
#[derive(Debug, Clone, Copy)]
pub struct ToonStyle3D {
    /// Width of the softened band edges in cosine space.
    pub softness: f32,
    /// Specular highlight strength on solid objects.
    pub specular: f32,
    /// Normal-map influence; low values keep painterly textures readable.
    pub normal_detail: f32,
    /// Multiplicative tint for the dark side of the diffuse ramp.
    pub shadow_tint: [f32; 3],
    /// Restrained lit rim strength, scaled by the scene's actual illumination.
    pub rim: f32,
}

impl Default for ToonStyle3D {
    fn default() -> Self {
        Self {
            softness: 0.035,
            specular: 0.12,
            normal_detail: 0.30,
            shadow_tint: [0.80, 0.87, 1.0],
            rim: 0.035,
        }
    }
}

impl ToonStyle3D {
    /// Four-level diffuse ramp. Kept in sync with `toon_diffuse` in WGSL.
    pub fn diffuse(self, cosine: f32) -> f32 {
        let x = cosine.clamp(0.0, 1.0);
        0.08 + 0.28 * self.edge(0.22, x) + 0.31 * self.edge(0.55, x) + 0.33 * self.edge(0.85, x)
    }

    /// Smooth threshold for a cel band or a shaped material highlight.
    pub fn edge(self, threshold: f32, value: f32) -> f32 {
        let width = self.softness.max(0.001);
        let t = ((value - threshold + width) / (2.0 * width)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ramp_is_bounded_and_monotonic_without_crushing_unlit_surfaces() {
        let style = ToonStyle3D::default();
        let mut previous = 0.0;
        for i in -100..=200 {
            let value = style.diffuse(i as f32 / 100.0);
            assert!((0.079..=1.001).contains(&value));
            assert!(value >= previous);
            previous = value;
        }
        assert_eq!(style.diffuse(0.0), style.diffuse(0.1));
        assert_eq!(style.diffuse(0.35), style.diffuse(0.45));
    }
}
