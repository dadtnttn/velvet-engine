//! RGBA preview rasterizer: hillshade + height tint, no image dependency.
//!
//! Returns raw row-major RGBA bytes; hosts encode (e.g. via `velvet-image`)
//! or blit directly. Used by the CLI and the `terraflow` demo.

use crate::graph::Heightfield;

/// Render a hillshade preview (`resolution²` RGBA pixels).
///
/// Height tints lowland green → rock brown → snow white; a fixed sun
/// direction modulates brightness through the central-difference normal.
pub fn preview_rgba(field: &Heightfield) -> Vec<u8> {
    let res = field.resolution as usize;
    let (lo, hi) = field.range();
    let span = (hi - lo).max(1e-6);
    let sun = normalize([-0.5, 0.8, 0.3]);
    let mut out = vec![0u8; res * res * 4];
    for j in 0..res {
        for i in 0..res {
            let h = field.at(i as i32, j as i32);
            let t = (h - lo) / span;
            let step = field.step();
            let dx = field.at(i as i32 + 1, j as i32) - field.at(i as i32 - 1, j as i32);
            let dz = field.at(i as i32, j as i32 + 1) - field.at(i as i32, j as i32 - 1);
            let n = normalize([-dx / (2.0 * step), 1.0, -dz / (2.0 * step)]);
            let shade = (n[0] * sun[0] + n[1] * sun[1] + n[2] * sun[2]).clamp(0.15, 1.0);
            // Lowland green → rock brown → snow white.
            let (r, g, b) = if t < 0.35 {
                (0.25 + 0.2 * t, 0.45 + 0.15 * t, 0.22)
            } else if t < 0.7 {
                (0.42, 0.33, 0.24)
            } else {
                (0.9, 0.9, 0.92)
            };
            let k = (j * res + i) * 4;
            out[k] = (r * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 1] = (g * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 2] = (b * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 3] = 255;
        }
    }
    out
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{TerrainGraph, TerrainParams};

    #[test]
    fn preview_covers_range_with_opaque_pixels() {
        let field = TerrainGraph::new(TerrainParams {
            seed: 9,
            resolution: 17,
            ..Default::default()
        })
        .bake();
        let px = preview_rgba(&field);
        assert_eq!(px.len(), 17 * 17 * 4);
        assert!(px.iter().skip(3).step_by(4).all(|a| *a == 255));
        // Not flat gray: min and max channels differ.
        let min = px.iter().min().copied().unwrap_or(0);
        let max = px.iter().max().copied().unwrap_or(0);
        assert!(max - min > 40, "{min}..{max}");
    }
}
