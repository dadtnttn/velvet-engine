//! # velvet-render-rtx
//!
//! Nvidia-specialized Vulkan RTX backend for Velvet Engine.
//!
//! This crate is **separate** from `velvet-render` on purpose:
//! - `velvet-render` keeps the cross-platform wgpu forward renderer (`Wgpu3DRenderer`,
//!   shadow maps, 16 point lights, PBR) that runs on any GPU.
//! - `velvet-render-rtx` owns hardware ray tracing via Vulkan
//!   `VK_KHR_ray_tracing_pipeline` + `VK_KHR_acceleration_structure` for Nvidia RTX.
//!
//! ## Design
//! - Trait `RenderBackend3D` abstracts both backends.
//! - Feature `rtx-nvidia` gates `ash` and all Vulkan RT code. Without it the crate
//!   compiles as a no-op stub, so CI / non-Nvidia machines still pass `cargo check`.
//! - Runtime detection: if no Nvidia RTX adapter or `rtx-nvidia` not enabled, fallback
//!   to `velvet-render::Wgpu3DRenderer`.

#![deny(missing_docs)]

pub mod backend;
pub mod capability;
#[cfg(feature = "rtx-nvidia")]
pub mod vulkan_rt;

pub use backend::{RenderBackend3D, RenderBackendKind, RtxError};
pub use capability::{RtxCapabilities, RtxSupportLevel};

#[cfg(feature = "rtx-nvidia")]
pub use vulkan_rt::VulkanRtxBackend;
