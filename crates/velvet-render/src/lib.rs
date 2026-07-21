//! # velvet-render
//!
//! Modern 2D renderer built on `wgpu`: sprites, cameras, virtual resolution,
//! batching, and named quality profiles.

#![deny(missing_docs)]

mod animation;
mod atlas;
mod batch;
mod camera;
mod culling;
mod debug_overlay;
mod gpu;
mod letterbox;
mod material;
mod particles;
mod plugin;
mod postprocess;
mod product_batch;
mod profile;
mod render_plan;
mod sort_key;
mod sprite;
mod stats;
mod texture;

pub mod prelude;

pub use animation::{AnimFrame, AnimLoop, SpriteAnimation};
pub use atlas::TextureAtlas;
pub use batch::{DrawCommand, SpriteBatch};
pub use camera::Camera2D;
pub use culling::{
    count_visible, cull_aabbs, expanded_visible_bounds, is_on_screen, CameraFrustum2D, CullResult,
};
pub use debug_overlay::{DebugOverlay, DebugShape, DebugTextLine};
pub use gpu::{GpuContext, GpuError, RenderSurface, SurfaceFrame};
pub use letterbox::{compute_letterbox, Letterbox, ScalingMode};
pub use material::{BlendMode, Material};
pub use particles::{Particle, ParticleBatch, ParticleEmitter, ParticleGpu};
pub use plugin::RenderPlugin;
pub use postprocess::{PostEffect, PostProcessStack};
pub use product_batch::{count_positive_quads, fill_batch_from_product_quads, ProductGpuQuad};
pub use profile::RenderProfile;
pub use render_plan::{CameraPass, ClearMode, RenderPlan, Viewport};
pub use sort_key::{
    build_sort_key, cmp_sort_keys, pack_z_texture, quantize_y_key, sort_by_key, SortLayers,
};
pub use sprite::{Sprite, SpriteFlip, SpriteInstance};
pub use stats::RenderStats;
pub use texture::{TextureId, TextureRegion, TextureStore};

/// Clear / background color used when no draw commands cover the framebuffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearColor {
    /// RGBA linear-ish components in 0..=1.
    pub color: velvet_math::Color,
}

impl Default for ClearColor {
    fn default() -> Self {
        Self {
            color: velvet_math::Color::rgb(0.08, 0.07, 0.12),
        }
    }
}

impl ClearColor {
    /// Create from RGBA.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: velvet_math::Color::rgba(r, g, b, a),
        }
    }

    /// As wgpu color.
    pub fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: f64::from(self.color.r),
            g: f64::from(self.color.g),
            b: f64::from(self.color.b),
            a: f64::from(self.color.a),
        }
    }
}
