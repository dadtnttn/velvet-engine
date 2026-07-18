//! CPU-side texture identifiers and atlases.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

static NEXT_TEXTURE_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque texture identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureId(u64);

impl TextureId {
    /// Invalid / none texture.
    pub const NONE: Self = Self(0);

    /// Allocate a new unique id (CPU bookkeeping; GPU upload separate).
    pub fn allocate() -> Self {
        Self(NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Raw value.
    pub fn raw(self) -> u64 {
        self.0
    }

    /// Whether this is none.
    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

impl Default for TextureId {
    fn default() -> Self {
        Self::NONE
    }
}

/// Sub-rectangle of a texture in **pixels**.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TextureRegion {
    /// Min pixel (left, top or bottom depending on convention — we use top-left UV origin).
    pub x: f32,
    /// Min y.
    pub y: f32,
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
}

impl TextureRegion {
    /// Full region helper once size known.
    pub fn full(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    /// Convert to UV rect given texture dimensions.
    pub fn to_uv(self, tex_w: f32, tex_h: f32) -> Rect {
        let tw = tex_w.max(1.0);
        let th = tex_h.max(1.0);
        Rect::from_min_max(
            Vec2::new(self.x / tw, self.y / th),
            Vec2::new((self.x + self.width) / tw, (self.y + self.height) / th),
        )
    }
}

/// Metadata for a loaded GPU texture (CPU mirror).
#[derive(Debug, Clone)]
pub struct TextureInfo {
    /// Id.
    pub id: TextureId,
    /// Width px.
    pub width: u32,
    /// Height px.
    pub height: u32,
    /// Debug label.
    pub label: String,
}

/// CPU registry of texture metadata (GPU objects live in [`crate::gpu::GpuContext`]).
#[derive(Debug, Default)]
pub struct TextureStore {
    infos: HashMap<TextureId, TextureInfo>,
}

impl TextureStore {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register metadata.
    pub fn register(&mut self, info: TextureInfo) {
        self.infos.insert(info.id, info);
    }

    /// Get info.
    pub fn get(&self, id: TextureId) -> Option<&TextureInfo> {
        self.infos.get(&id)
    }

    /// Remove.
    pub fn remove(&mut self, id: TextureId) -> Option<TextureInfo> {
        self.infos.remove(&id)
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.infos.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.infos.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_to_uv() {
        let r = TextureRegion {
            x: 16.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        };
        let uv = r.to_uv(64.0, 16.0);
        assert!((uv.min.x - 0.25).abs() < 1e-5);
        assert!((uv.max.x - 0.5).abs() < 1e-5);
    }
}
