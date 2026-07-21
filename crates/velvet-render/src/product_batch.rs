//! Bridge product paint descriptors → [`SpriteBatch`] for the wgpu presenter.
//!
//! Hosts pass center/size/color quads (from `velvet-story::RenderDrawDescriptor`)
//! without depending on story types inside this crate.

use velvet_math::{Color, Transform2D, Vec2};

use crate::batch::SpriteBatch;
use crate::texture::TextureId;

/// Minimal product quad for GPU submission (matches story render descriptors).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProductGpuQuad {
    /// Center x (virtual resolution).
    pub cx: f32,
    /// Center y.
    pub cy: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// RGBA 0..=1.
    pub color: [f32; 4],
    /// Z / layer.
    pub z: f32,
}

impl ProductGpuQuad {
    /// From center, size, color, z.
    pub fn new(cx: f32, cy: f32, w: f32, h: f32, color: [f32; 4], z: f32) -> Self {
        Self {
            cx,
            cy,
            w,
            h,
            color,
            z,
        }
    }

    /// True when drawable.
    pub fn is_positive(&self) -> bool {
        self.w > 0.0 && self.h > 0.0
    }
}

/// Push colored product quads into a sprite batch using a white (or solid) texture.
pub fn fill_batch_from_product_quads(
    batch: &mut SpriteBatch,
    white: TextureId,
    quads: &[ProductGpuQuad],
) -> usize {
    let mut n = 0usize;
    for q in quads {
        if !q.is_positive() {
            continue;
        }
        let tint = Color::rgba(q.color[0], q.color[1], q.color[2], q.color[3]);
        let transform = Transform2D::from_translation(Vec2::new(q.cx, q.cy));
        batch.push_colored_quad(white, transform, Vec2::new(q.w, q.h), tint, q.z);
        n += 1;
    }
    n
}

/// Count how many quads would be submitted.
pub fn count_positive_quads(quads: &[ProductGpuQuad]) -> usize {
    quads.iter().filter(|q| q.is_positive()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::SpriteBatch;
    use crate::camera::Camera2D;
    use crate::gpu::GpuContext;
    use velvet_math::Vec2;

    #[test]
    fn fill_batch_counts_positive_only() {
        let mut batch = SpriteBatch::new();
        let white = TextureId::NONE;
        let quads = [
            ProductGpuQuad::new(100.0, 100.0, 200.0, 80.0, [1.0, 0.0, 0.0, 1.0], 1.0),
            ProductGpuQuad::new(0.0, 0.0, 0.0, 10.0, [1.0, 1.0, 1.0, 1.0], 0.0),
            ProductGpuQuad::new(50.0, 50.0, 40.0, 40.0, [0.0, 1.0, 0.0, 0.5], 2.0),
        ];
        let n = fill_batch_from_product_quads(&mut batch, white, &quads);
        assert_eq!(n, 2);
        assert_eq!(batch.len(), 2);
        assert_eq!(count_positive_quads(&quads), 2);
    }

    #[test]
    fn headless_gpu_present_product_quads() {
        let mut gpu = match GpuContext::headless() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("phase1_gpu_env: skip headless present: {e}");
                return;
            }
        };
        gpu.virtual_size = Vec2::new(1280.0, 720.0);
        let white = gpu.white_texture;
        let mut batch = SpriteBatch::new();
        let quads = [
            ProductGpuQuad::new(640.0, 360.0, 1280.0, 720.0, [0.1, 0.1, 0.2, 1.0], 0.0),
            ProductGpuQuad::new(640.0, 600.0, 900.0, 140.0, [0.08, 0.09, 0.14, 0.9], 10.0),
        ];
        let n = fill_batch_from_product_quads(&mut batch, white, &quads);
        assert_eq!(n, 2);

        // Offscreen target
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("product-present-test"),
            size: wgpu::Extent3d {
                width: 128,
                height: 72,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let cam = Camera2D::virtual_res(1280.0, 720.0);
        gpu.render_batch(&view, (128, 72), &cam, &mut batch);
        assert!(batch.len() >= 2);
        eprintln!(
            "phase1_gpu_env: ok adapter={} sprites_in_batch={}",
            gpu.adapter_info, n
        );
    }
}
