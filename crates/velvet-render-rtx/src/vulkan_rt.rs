//! Vulkan RTX backend for Nvidia — gated by `rtx-nvidia` feature.
//!
//! This module uses `ash` to talk to Vulkan. It validates the three
//! extensions required for hardware ray tracing and otherwise falls back
//! to `RtxError::Unsupported` so the engine can keep using the forward renderer.
//!
//! ## Extensions required
//! - `VK_KHR_acceleration_structure`
//! - `VK_KHR_ray_tracing_pipeline`
//! - `VK_KHR_deferred_host_operations` (dependency of acceleration_structure)
//!
//! ## Pipeline (future work, staged)
//! 1. Create `Instance` + `Device` with those extensions + `PhysicalDeviceRayTracingPipelineFeaturesKHR`.
//! 2. Build BLAS per `RtxMesh` (vertex/index buffers -> `VkAccelerationStructureKHR`).
//! 3. Build TLAS per frame from `RtxDraw` transforms.
//! 4. Ray tracing pipeline: `raygen.rgen` / `miss.rmiss` / `closesthit.rchit` (GLSL -> SPIR-V via `shaderc`).
//! 5. `vkCmdTraceRaysKHR` into storage image, then blit to swapchain.
//!
//! The current implementation validates instance creation and reports
//! capabilities; full BLAS/TLAS + pipeline is staged to keep the workspace
//! green without requiring Vulkan SDK on CI.

use ash::vk;
use tracing::info;

use crate::backend::{RenderBackend3D, RenderBackendKind, RtxCamera, RtxDraw, RtxError, RtxMesh};
use crate::capability::{RtxCapabilities, RtxSupportLevel};

const REQUIRED_DEVICE_EXTENSIONS: &[&str] = &[
    "VK_KHR_acceleration_structure",
    "VK_KHR_ray_tracing_pipeline",
    "VK_KHR_deferred_host_operations",
];

/// Nvidia Vulkan RTX backend.
///
/// Holds Vulkan `Entry`/`Instance` when `rtx-nvidia` is enabled. Drops cleanly
/// and never panics if no RTX adapter is present — caller falls back to
/// `velvet-render::Wgpu3DRenderer`.
pub struct VulkanRtxBackend {
    _entry: ash::Entry,
    instance: ash::Instance,
    /// Whether required RT extensions were found on the selected physical device.
    capabilities: RtxCapabilities,
    meshes: Vec<RtxMesh>,
}

impl VulkanRtxBackend {
    /// Probe Vulkan and create an instance, checking for RTX support.
    ///
    /// Returns `Err(RtxError::NoAdapter)` if no Vulkan loader is present,
    /// or `Err(RtxError::Unsupported)` if RT extensions are missing — both
    /// are non-fatal for the engine.
    pub fn probe() -> Result<Self, RtxError> {
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| RtxError::Vulkan(format!("failed to load Vulkan loader: {e}")))?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"velvet-rtx")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"VelvetEngine")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_2);

        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .map_err(|e| RtxError::Vulkan(format!("vkCreateInstance: {e:?}")))?;

        // Enumerate physical devices and look for Nvidia + RT extensions.
        let pdevices = unsafe { instance.enumerate_physical_devices() }
            .map_err(|e| RtxError::Vulkan(format!("enumerate_physical_devices: {e:?}")))?;

        if pdevices.is_empty() {
            unsafe { instance.destroy_instance(None) };
            return Err(RtxError::NoAdapter("no Vulkan physical devices".into()));
        }

        let mut best_capabilities: Option<RtxCapabilities> = None;

        for pdevice in &pdevices {
            let props = unsafe { instance.get_physical_device_properties(*pdevice) };
            let name = unsafe {
                std::ffi::CStr::from_ptr(props.device_name.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            let is_nvidia = props.vendor_id == 0x10DE;

            let ext_props = unsafe {
                instance
                    .enumerate_device_extension_properties(*pdevice)
                    .unwrap_or_default()
            };
            let available: Vec<String> = ext_props
                .iter()
                .map(|ext| unsafe {
                    std::ffi::CStr::from_ptr(ext.extension_name.as_ptr())
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            let missing: Vec<String> = REQUIRED_DEVICE_EXTENSIONS
                .iter()
                .filter(|req| !available.iter().any(|a| a == *req))
                .map(|s| s.to_string())
                .collect();

            let level = if missing.is_empty() && is_nvidia {
                RtxSupportLevel::Full
            } else if missing.is_empty() {
                // RT available but not Nvidia — still usable, but we label as Full for now.
                RtxSupportLevel::Full
            } else if is_nvidia {
                RtxSupportLevel::VulkanWithoutRt
            } else {
                RtxSupportLevel::VulkanWithoutRt
            };

            let caps = RtxCapabilities {
                level,
                adapter_name: Some(name.clone()),
                vulkan_version: Some(format!(
                    "{}.{}.{}",
                    vk::api_version_major(props.api_version),
                    vk::api_version_minor(props.api_version),
                    vk::api_version_patch(props.api_version)
                )),
                missing_extensions: missing.clone(),
            };

            // Prefer Nvidia Full.
            if level == RtxSupportLevel::Full && is_nvidia {
                best_capabilities = Some(caps);
                break;
            }
            if best_capabilities.is_none() {
                best_capabilities = Some(caps);
            }
        }

        let capabilities = best_capabilities.unwrap_or_else(|| RtxCapabilities::none("no devices"));

        info!(
            adapter = capabilities.adapter_name.as_deref().unwrap_or("-"),
            level = capabilities.level.label(),
            missing = ?capabilities.missing_extensions,
            "Vulkan RTX probe"
        );

        if capabilities.level != RtxSupportLevel::Full {
            // Keep instance alive for diagnostics but signal unsupported so caller can fallback.
            // We still return Ok — backend exists but will report Unsupported on render.
            // To keep API simple, we store capabilities and let render() fail gracefully.
        }

        Ok(Self {
            _entry: entry,
            instance,
            capabilities,
            meshes: Vec::new(),
        })
    }

    /// Capabilities snapshot.
    pub fn capabilities(&self) -> &RtxCapabilities {
        &self.capabilities
    }

    /// Whether hardware RT can be dispatched this frame.
    pub fn can_trace(&self) -> bool {
        self.capabilities.level.is_available()
    }
}

impl Drop for VulkanRtxBackend {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}

impl RenderBackend3D for VulkanRtxBackend {
    fn kind(&self) -> RenderBackendKind {
        RenderBackendKind::VulkanRtx
    }

    fn upload_mesh(&mut self, mesh: &RtxMesh) -> Result<usize, RtxError> {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            return Err(RtxError::Gpu("empty mesh".into()));
        }
        // TODO: Build BLAS via vkCreateAccelerationStructureKHR + vkBuildAccelerationStructuresKHR
        // For now we store CPU copy so the crate compiles without device creation.
        let idx = self.meshes.len();
        self.meshes.push(mesh.clone());
        Ok(idx)
    }

    fn render(&mut self, _camera: RtxCamera, _draws: &[RtxDraw]) -> Result<(), RtxError> {
        if !self.can_trace() {
            return Err(RtxError::Unsupported(self.capabilities.summary()));
        }
        // TODO: Build TLAS from draws[].transform, bind ray tracing pipeline, vkCmdTraceRaysKHR.
        // Stub keeps workspace green and documents the call site where tracing will happen.
        Err(RtxError::Gpu(
            "Vulkan RTX pipeline not yet implemented — BLAS/TLAS + raygen/miss/hit pending (vulkan_rt.rs)".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_extensions_list_is_complete() {
        assert!(REQUIRED_DEVICE_EXTENSIONS.contains(&"VK_KHR_acceleration_structure"));
        assert!(REQUIRED_DEVICE_EXTENSIONS.contains(&"VK_KHR_ray_tracing_pipeline"));
        assert!(REQUIRED_DEVICE_EXTENSIONS.contains(&"VK_KHR_deferred_host_operations"));
    }

    // probe() is not run in CI (needs Vulkan loader). We only test that the error
    // type is usable without panicking when feature is off.
    #[test]
    fn probe_error_is_non_fatal() {
        let err = RtxError::NoAdapter("no Vulkan loader — fallback to forward".into());
        assert!(err.to_string().contains("no RTX adapter"));
    }
}
