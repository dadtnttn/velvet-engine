//! Render prelude.

pub use crate::animation::{AnimFrame, AnimLoop, SpriteAnimation};
pub use crate::atlas::TextureAtlas;
pub use crate::batch::{DrawCommand, SpriteBatch};
pub use crate::camera::Camera2D;
pub use crate::debug_overlay::{DebugOverlay, DebugShape};
pub use crate::gpu::{GpuContext, GpuError};
pub use crate::letterbox::{compute_letterbox, Letterbox, ScalingMode};
pub use crate::mesh3d::{
    AnimationClip3D, Camera3D, GltfModel, Lighting3D, Material3D, Mesh3D, MeshDraw3D, MeshHandle,
    MeshVertex3D, Render3DError, SkinWeights3D, TextureData3D, TextureHandle, Transform3D,
    Wgpu3DRenderer,
};
pub use crate::particles::{Particle, ParticleBatch};
pub use crate::plugin::{RenderConfig, RenderFrame, RenderPlugin};
pub use crate::postprocess::{PostEffect, PostProcessStack};
pub use crate::product_batch::{
    count_positive_quads, fill_batch_from_product_quads, ProductGpuQuad,
};
pub use crate::profile::RenderProfile;
pub use crate::software::{SoftwarePresenter, SoftwareScalingFilter, SoftwareViewport};
pub use crate::sprite::{Sprite, SpriteFlip, SpriteInstance};
pub use crate::stats::RenderStats;
pub use crate::texture::{TextureId, TextureRegion, TextureStore};
pub use crate::ClearColor;
