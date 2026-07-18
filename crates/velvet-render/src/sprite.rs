//! Sprite draw primitives.

use serde::{Deserialize, Serialize};
use velvet_math::{Color, Rect, Transform2D, Vec2};

use crate::texture::{TextureId, TextureRegion};

/// Flip flags for sprites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct SpriteFlip {
    /// Flip horizontally.
    pub x: bool,
    /// Flip vertically.
    pub y: bool,
}

impl SpriteFlip {
    /// No flip.
    pub const NONE: Self = Self { x: false, y: false };
    /// Horizontal only.
    pub const X: Self = Self { x: true, y: false };
    /// Vertical only.
    pub const Y: Self = Self { x: false, y: true };
}

/// Logical sprite description (CPU-side).
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// Texture handle.
    pub texture: TextureId,
    /// Optional sub-rectangle in texture pixels (None = full texture).
    pub region: Option<TextureRegion>,
    /// World transform.
    pub transform: Transform2D,
    /// Size in world units (if None, use region/texture pixel size as world size).
    pub size: Option<Vec2>,
    /// Multiplicative tint.
    pub tint: Color,
    /// Sort / z layer (higher drawn later).
    pub z: f32,
    /// Flip.
    pub flip: SpriteFlip,
    /// Anchor in 0..=1 relative to size (0.5,0.5 = center).
    pub anchor: Vec2,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture: TextureId::NONE,
            region: None,
            transform: Transform2D::IDENTITY,
            size: Some(Vec2::ONE),
            tint: Color::WHITE,
            z: 0.0,
            flip: SpriteFlip::NONE,
            anchor: Vec2::new(0.5, 0.5),
        }
    }
}

impl Sprite {
    /// Simple full-texture sprite at position.
    pub fn at(texture: TextureId, position: Vec2, size: Vec2) -> Self {
        Self {
            texture,
            transform: Transform2D::from_translation(position),
            size: Some(size),
            ..Default::default()
        }
    }

    /// World-space axis-aligned bounds (ignores rotation for coarse AABB).
    pub fn approx_aabb(&self) -> Rect {
        let size = self.size.unwrap_or(Vec2::ONE);
        let origin = self.transform.translation
            - Vec2::new(size.x * self.anchor.x, size.y * self.anchor.y) * self.transform.scale;
        // Scale only for AABB approx.
        let scaled = size * self.transform.scale;
        Rect::from_pos_size(origin, scaled)
    }
}

/// GPU-ready instance data (tightly packed).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    /// Column-major 3x3 affine as 3 vec3 (pad w unused) — we pack as 4x f32 rows for simplicity:
    /// transform columns xy + translation in [0..6], uv rect [6..10], tint [10..14], z + pad.
    pub data: [f32; 16],
}

impl SpriteInstance {
    /// Build from sprite fields and UV rect in 0..=1.
    pub fn from_parts(
        transform: Transform2D,
        size: Vec2,
        anchor: Vec2,
        uv: Rect,
        tint: Color,
        z: f32,
        flip: SpriteFlip,
    ) -> Self {
        let mut uv_min_x = uv.min.x;
        let mut uv_min_y = uv.min.y;
        let mut uv_max_x = uv.max.x;
        let mut uv_max_y = uv.max.y;
        if flip.x {
            std::mem::swap(&mut uv_min_x, &mut uv_max_x);
        }
        if flip.y {
            std::mem::swap(&mut uv_min_y, &mut uv_max_y);
        }

        // Pack: mat cols (9) + size (2) + anchor (2) + z + pad → use 16 floats:
        // [m00,m01,m02, m10,m11,m12, m20,m21, size.x, size.y, anchor.x, anchor.y, uv...]
        // Simpler packing for shader:
        // data[0..2] translation, [2] rotation, [3] unused
        // data[4..6] scale * size, [6..8] anchor
        // data[8..12] uv min/max
        // data[12..16] tint
        Self {
            data: [
                transform.translation.x,
                transform.translation.y,
                transform.rotation,
                z,
                transform.scale.x * size.x,
                transform.scale.y * size.y,
                anchor.x,
                anchor.y,
                uv_min_x,
                uv_min_y,
                uv_max_x,
                uv_max_y,
                tint.r,
                tint.g,
                tint.b,
                tint.a,
            ],
        }
    }

    /// Z used for sorting.
    pub fn z(self) -> f32 {
        self.data[3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_flip_swaps_uv() {
        let uv = Rect::from_pos_size(Vec2::ZERO, Vec2::ONE);
        let a = SpriteInstance::from_parts(
            Transform2D::IDENTITY,
            Vec2::ONE,
            Vec2::new(0.5, 0.5),
            uv,
            Color::WHITE,
            0.0,
            SpriteFlip::X,
        );
        assert!((a.data[8] - 1.0).abs() < 1e-5);
        assert!((a.data[10] - 0.0).abs() < 1e-5);
    }
}
