//! Abstraction over forward vs RTX backends.

use thiserror::Error;
use velvet_math::{Mat4, Vec3};

/// Errors from RTX backend creation / rendering.
#[derive(Debug, Error)]
pub enum RtxError {
    /// Vulkan instance / device creation failed.
    #[error("vulkan: {0}")]
    Vulkan(String),
    /// Required RTX extensions not supported by adapter.
    #[error("RTX not supported: {0}")]
    Unsupported(String),
    /// No Nvidia RTX adapter found (fallback should be used).
    #[error("no RTX adapter: {0}")]
    NoAdapter(String),
    /// Generic GPU error.
    #[error("gpu: {0}")]
    Gpu(String),
}

/// Which backend is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackendKind {
    /// Cross-platform wgpu forward renderer (`velvet-render::Wgpu3DRenderer`).
    ForwardWgpu,
    /// Nvidia Vulkan RTX hardware ray tracing.
    VulkanRtx,
}

impl RenderBackendKind {
    /// Human label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ForwardWgpu => "Forward wgpu",
            Self::VulkanRtx => "Vulkan RTX (Nvidia)",
        }
    }
}

/// Minimal mesh description shared between backends (mirrors `velvet-render::Mesh3D` without wgpu coupling).
#[derive(Debug, Clone)]
pub struct RtxMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Normals.
    pub normals: Vec<[f32; 3]>,
    /// UVs.
    pub uvs: Vec<[f32; 2]>,
    /// Indices.
    pub indices: Vec<u32>,
}

/// Instance draw.
#[derive(Debug, Clone, Copy)]
pub struct RtxDraw {
    /// Mesh index in uploaded list.
    pub mesh_index: usize,
    /// World transform.
    pub transform: Mat4,
}

/// Simple camera for RTX (mirrors `velvet-render::Camera3D` but decoupled).
#[derive(Debug, Clone, Copy)]
pub struct RtxCamera {
    /// Position.
    pub position: Vec3,
    /// Target.
    pub target: Vec3,
    /// Up.
    pub up: Vec3,
    /// Vertical FOV radians.
    pub fov_y: f32,
    /// Near.
    pub near: f32,
    /// Far.
    pub far: f32,
}

impl Default for RtxCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(4.5, 3.2, 5.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            up: Vec3::Y,
            fov_y: 45.0_f32.to_radians(),
            near: 0.05,
            far: 200.0,
        }
    }
}

/// Abstraction implemented by both backends.
pub trait RenderBackend3D {
    /// Backend kind.
    fn kind(&self) -> RenderBackendKind;

    /// Whether hardware RTX is actually active (false for forward fallback).
    fn is_hardware_rt(&self) -> bool {
        self.kind() == RenderBackendKind::VulkanRtx
    }

    /// Upload a mesh and return its index. Implementations validate topology.
    fn upload_mesh(&mut self, mesh: &RtxMesh) -> Result<usize, RtxError>;

    /// Record / submit a frame. Called once per frame after all uploads.
    fn render(&mut self, camera: RtxCamera, draws: &[RtxDraw]) -> Result<(), RtxError>;
}

/// No-op fallback used when `rtx-nvidia` feature is off or no adapter.
#[derive(Debug, Default)]
pub struct FallbackBackend {
    meshes: Vec<RtxMesh>,
}

impl RenderBackend3D for FallbackBackend {
    fn kind(&self) -> RenderBackendKind {
        RenderBackendKind::ForwardWgpu
    }

    fn upload_mesh(&mut self, mesh: &RtxMesh) -> Result<usize, RtxError> {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            return Err(RtxError::Gpu("empty mesh".into()));
        }
        let idx = self.meshes.len();
        self.meshes.push(mesh.clone());
        Ok(idx)
    }

    fn render(&mut self, _camera: RtxCamera, _draws: &[RtxDraw]) -> Result<(), RtxError> {
        // Forward path is owned by velvet-render::Wgpu3DRenderer; this stub just validates.
        Ok(())
    }
}
