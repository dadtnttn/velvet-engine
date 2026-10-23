//! # velvet-render
//!
//! Modern 2D/3D renderer built on `wgpu`: sprites, meshes, cameras, virtual
//! resolution, batching, depth testing, and named quality profiles.

#![deny(missing_docs)]

/// CPU time spent in the 3D renderer's frame calls (nanoseconds, summed): for profiling.
pub static RENDER3D_CPU_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
mod animation;
mod atlas;
mod batch;
mod bc7;
mod camera;
pub mod crowd3d;
mod culling;
mod debug_overlay;
mod gi3d;
mod gpu;
mod iso;
mod letterbox;
mod material;
mod mesh3d;
mod particles;
mod plugin;
mod post3d;
mod postprocess;
mod product_batch;
mod profile;
mod readback;
mod render_plan;
/// Clips from one rig onto another (bone maps, rest alignment, quaternion helpers).
pub mod retarget;
pub mod scene_bridge;
mod skin3d;
mod software;
mod sort_key;
mod sprite;
mod stats;
mod texture;
mod toon;
mod toon_fire;
mod upscale3d;
pub use upscale3d::{Antialias3D, Finishing3D, Upscale3D, Upscaler3D};
mod water3d;
mod water_life;
pub mod work;

pub mod prelude;

pub use animation::{AnimFrame, AnimLoop, SpriteAnimation};
pub use atlas::TextureAtlas;
pub use batch::{DrawCommand, SpriteBatch};
pub use camera::Camera2D;
pub use crowd3d::{CrowdInstance3D, CrowdMesh3D, CrowdVertex3D};
pub use culling::{
    count_visible, cull_aabbs, expanded_visible_bounds, is_on_screen, CameraFrustum2D, CullResult,
};
pub use debug_overlay::{DebugOverlay, DebugShape, DebugTextLine};
pub use gpu::{GpuContext, GpuError, RenderSurface, SurfaceFrame};
pub use iso::{CutawayWalls3D, IsoCamera3D, IsoSettings3D, PlanRoom3D, PlanWall3D};
pub use letterbox::{compute_letterbox, Letterbox, ScalingMode};
pub use material::{BlendMode, Material};
pub use mesh3d::{image_original, set_image_override, unpack_model_bytes, ImageOverride};
pub use mesh3d::{
    AnimatedMesh3D, AnimationClip3D, Atmosphere3D, Camera3D, Celestial3D, DayNightCycle3D,
    EnvironmentFrame3D, EnvironmentPreset3D, Finish3D, GltfModel, Lighting3D, Material3D, Mesh3D,
    MeshDraw3D, MeshHandle, MeshVertex3D, PointLight3D, PreparedTexture3D, Quality3D,
    Render3DError, RotationTracks3D, SkinWeights3D, TextureData3D, TextureHandle, Transform3D,
    Wgpu3DRenderer,
};
pub use particles::{Particle, ParticleBatch, ParticleEmitter, ParticleGpu};
pub use plugin::RenderPlugin;
pub use postprocess::{PostEffect, PostProcessStack};
pub use product_batch::{count_positive_quads, fill_batch_from_product_quads, ProductGpuQuad};
pub use profile::RenderProfile;
pub use readback::{frame_is_blank, read_texture_rgba};
pub use render_plan::{CameraPass, ClearMode, RenderPlan, Viewport};
pub use scene_bridge::{material_from_doc, scene_draws, upload_scene_meshes, SceneMeshCache};
pub use skin3d::{Animator3D, Pose3D};
pub use software::{SoftwarePresenter, SoftwareScalingFilter, SoftwareViewport};
pub use sort_key::{
    build_sort_key, cmp_sort_keys, pack_z_texture, quantize_y_key, sort_by_key, SortLayers,
};
pub use sprite::{Sprite, SpriteFlip, SpriteInstance};
pub use stats::RenderStats;
pub use texture::{TextureId, TextureRegion, TextureStore};
pub use toon::{ShadingStyle3D, ToonStyle3D};
pub use toon_fire::ToonFire3D;
pub use water3d::{
    WaterBody3D, WaterBodyHandle, WaterLook3D, WaterStream3D, WaterStreamHandle, WaterStyle3D,
};
pub use water_life::WaterLife3D;

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
