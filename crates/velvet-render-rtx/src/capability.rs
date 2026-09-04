//! RTX capability detection (decoupled from Vulkan instance creation).

/// How well RTX is supported on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtxSupportLevel {
    /// No Nvidia RTX adapter or extensions missing — use forward fallback.
    None,
    /// Vulkan available but RT extensions not present.
    VulkanWithoutRt,
    /// Full `VK_KHR_acceleration_structure` + `VK_KHR_ray_tracing_pipeline` + `VK_KHR_deferred_host_operations`.
    Full,
}

impl RtxSupportLevel {
    /// Whether hardware RT can be used.
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Label for logs / UI.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No RTX adapter",
            Self::VulkanWithoutRt => "Vulkan without RT",
            Self::Full => "Vulkan RTX ready (Nvidia)",
        }
    }
}

/// Snapshot of capabilities for diagnostics.
#[derive(Debug, Clone)]
pub struct RtxCapabilities {
    /// Support level.
    pub level: RtxSupportLevel,
    /// Adapter name if found.
    pub adapter_name: Option<String>,
    /// Vulkan API version.
    pub vulkan_version: Option<String>,
    /// Missing extensions if any.
    pub missing_extensions: Vec<String>,
}

impl RtxCapabilities {
    /// No support.
    pub fn none(reason: impl Into<String>) -> Self {
        Self {
            level: RtxSupportLevel::None,
            adapter_name: None,
            vulkan_version: None,
            missing_extensions: vec![reason.into()],
        }
    }

    /// Human summary.
    pub fn summary(&self) -> String {
        format!(
            "{} | adapter={} | missing={:?}",
            self.level.label(),
            self.adapter_name.as_deref().unwrap_or("-"),
            self.missing_extensions
        )
    }
}
