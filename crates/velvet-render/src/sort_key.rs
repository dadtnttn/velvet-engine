//! Expanded batch sorting keys for 2D draw commands.

use crate::texture::TextureId;

/// Layers of the sort key (high → low significance when packing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SortLayers {
    /// Camera / view layer (multi-camera). Higher drawn later by default.
    pub camera: u16,
    /// Logical render layer (background, world, y-sort, ui…).
    pub layer: i16,
    /// Y-sort key (screen/world y quantized); higher = in front when y-up world with inverted key.
    pub y_key: u16,
    /// Transparency bucket: 0 opaque, 1 alpha-test, 2 transparent.
    pub transparency: u8,
    /// Material / pipeline variant.
    pub material: u8,
    /// Texture id low bits for batching.
    pub texture: u32,
    /// Stable insertion order tie-breaker.
    pub order: u16,
}

impl SortLayers {
    /// Opaque world sprite defaults.
    pub fn opaque(layer: i16, texture: TextureId, order: u16) -> Self {
        Self {
            camera: 0,
            layer,
            y_key: 0,
            transparency: 0,
            material: 0,
            texture: texture.raw() as u32,
            order,
        }
    }

    /// Transparent sprite (sorted back-to-front via y/order).
    pub fn transparent(layer: i16, y_key: u16, texture: TextureId, order: u16) -> Self {
        Self {
            camera: 0,
            layer,
            y_key,
            transparency: 2,
            material: 0,
            texture: texture.raw() as u32,
            order,
        }
    }

    /// UI layer.
    pub fn ui(sublayer: i16, texture: TextureId, order: u16) -> Self {
        Self {
            camera: 0,
            layer: 10_000 + sublayer,
            y_key: 0,
            transparency: 2,
            material: 0,
            texture: texture.raw() as u32,
            order,
        }
    }

    /// Pack into a single `u64` for fast sort (priority left-to-right).
    ///
    /// Layout (MSB → LSB):
    /// - camera: 8 bits
    /// - layer: 12 bits (biased i16)
    /// - transparency: 2 bits
    /// - material: 6 bits
    /// - y_key: 12 bits
    /// - texture: 16 bits
    /// - order: 8 bits
    pub fn pack(self) -> u64 {
        let cam = (self.camera as u64) & 0xFF;
        let layer = ((self.layer as i32 + 2048).clamp(0, 4095) as u64) & 0xFFF;
        let transp = (self.transparency as u64) & 0x3;
        let mat = (self.material as u64) & 0x3F;
        let y = (self.y_key as u64) & 0xFFF;
        let tex = (self.texture as u64) & 0xFFFF;
        let order = (self.order as u64) & 0xFF;
        (cam << 56) | (layer << 44) | (transp << 42) | (mat << 36) | (y << 24) | (tex << 8) | order
    }

    /// Unpack from packed key (lossy for fields larger than bit budget).
    pub fn unpack(key: u64) -> Self {
        let cam = ((key >> 56) & 0xFF) as u16;
        let layer_u = ((key >> 44) & 0xFFF) as i32;
        let layer = (layer_u - 2048) as i16;
        let transparency = ((key >> 42) & 0x3) as u8;
        let material = ((key >> 36) & 0x3F) as u8;
        let y_key = ((key >> 24) & 0xFFF) as u16;
        let texture = ((key >> 8) & 0xFFFF) as u32;
        let order = (key & 0xFF) as u16;
        Self {
            camera: cam,
            layer,
            y_key,
            transparency,
            material,
            texture,
            order,
        }
    }
}

/// Quantize a world/screen Y into a 12-bit key for y-sorting.
pub fn quantize_y_key(y: f32, min_y: f32, max_y: f32) -> u16 {
    let span = (max_y - min_y).max(1e-3);
    let t = ((y - min_y) / span).clamp(0.0, 1.0);
    (t * 4095.0) as u16
}

/// Pack classic z + texture key (compatible with previous batcher).
pub fn pack_z_texture(z: f32, texture: TextureId) -> u64 {
    let z_bits = (z.to_bits() as u64) << 32;
    z_bits | (texture.raw() & 0xFFFF_FFFF)
}

/// Build a sort key from camera, layer, optional y-sort, z, and texture.
pub fn build_sort_key(
    camera: u16,
    layer: i16,
    y: Option<f32>,
    y_range: (f32, f32),
    transparent: bool,
    texture: TextureId,
    order: u16,
) -> u64 {
    let y_key = y
        .map(|yy| quantize_y_key(yy, y_range.0, y_range.1))
        .unwrap_or(0);
    SortLayers {
        camera,
        layer,
        y_key,
        transparency: if transparent { 2 } else { 0 },
        material: 0,
        texture: texture.raw() as u32,
        order,
    }
    .pack()
}

/// Compare two packed keys (ascending = back to front for our pack layout).
#[inline]
pub fn cmp_sort_keys(a: u64, b: u64) -> std::cmp::Ordering {
    a.cmp(&b)
}

/// Stable sort commands by key using the provided key extractor.
pub fn sort_by_key<T>(items: &mut [T], key_of: impl Fn(&T) -> u64) {
    items.sort_by_cached_key(|item| key_of(item));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_order_camera_first() {
        let mut a = SortLayers::opaque(0, TextureId::allocate(), 0);
        a.camera = 0;
        let mut b = SortLayers::opaque(0, TextureId::allocate(), 0);
        b.camera = 1;
        assert!(a.pack() < b.pack());
    }

    #[test]
    fn pack_layer_orders() {
        let t = TextureId::allocate();
        let back = SortLayers::opaque(-10, t, 0).pack();
        let front = SortLayers::opaque(10, t, 0).pack();
        assert!(back < front);
    }

    #[test]
    fn y_sort_transparent() {
        let t = TextureId::allocate();
        let near = SortLayers::transparent(0, 100, t, 0).pack();
        let far = SortLayers::transparent(0, 10, t, 0).pack();
        assert!(far < near);
    }

    #[test]
    fn unpack_roundtrip_small() {
        let t = TextureId::allocate();
        let s = SortLayers {
            camera: 3,
            layer: 5,
            y_key: 100,
            transparency: 2,
            material: 4,
            texture: (t.raw() as u32) & 0xFFFF,
            order: 7,
        };
        let u = SortLayers::unpack(s.pack());
        assert_eq!(u.camera, s.camera);
        assert_eq!(u.layer, s.layer);
        assert_eq!(u.y_key, s.y_key);
        assert_eq!(u.transparency, s.transparency);
        assert_eq!(u.material, s.material);
        assert_eq!(u.order, s.order);
    }

    #[test]
    fn quantize_y() {
        assert_eq!(quantize_y_key(0.0, 0.0, 100.0), 0);
        assert_eq!(quantize_y_key(100.0, 0.0, 100.0), 4095);
    }
}
