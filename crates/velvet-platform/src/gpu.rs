//! The GPU side of the platform: which wgpu backends to use, an adapter that can draw into the
//! window, limits a phone can give, and a surface that survives Android's suspend and resume.
//!
//! On Android the window's surface (an `ANativeWindow`) is destroyed every time the app leaves
//! the screen, and a new one comes with the next `resumed`. The device, queue, pipelines and
//! textures stay valid; only the `wgpu::Surface` must be dropped and made again from the same
//! `wgpu::Instance`. [`SurfaceSlot`] does that and otherwise behaves like a `wgpu::Surface`
//! (`configure`, `get_current_texture`), so a desktop renderer needs few changes.

use std::sync::Arc;

use winit::window::Window;

/// The wgpu instance descriptor for this platform; `WGPU_BACKEND` and the other `WGPU_*`
/// variables still override it.
///
/// On Android it enables one backend: Vulkan when the device has a hardware Vulkan adapter, else
/// OpenGL ES. An instance with both makes a surface for each, and Android's Vulkan loader
/// connects the window when it makes its surface, so EGL can no longer draw into it ("already
/// connected to another API", then wgpu's "Invalid surface") - even on devices with no Vulkan
/// driver. A software Vulkan (SwiftShader, as on emulators) counts as none: it froze the
/// emulator, whose OpenGL ES translator works. Elsewhere: wgpu's defaults.
pub fn instance_descriptor() -> wgpu::InstanceDescriptor {
    #[allow(unused_mut)]
    let mut descriptor = wgpu::InstanceDescriptor::default();
    #[cfg(target_os = "android")]
    {
        descriptor.backends = android_backend();
    }
    descriptor.with_env()
}

/// Vulkan if a probe instance finds a Vulkan adapter that is not a CPU (software) device, else
/// OpenGL ES.
#[cfg(target_os = "android")]
fn android_backend() -> wgpu::Backends {
    let probe = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let hardware_vulkan = probe
        .enumerate_adapters(wgpu::Backends::VULKAN)
        .iter()
        .any(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu);
    if hardware_vulkan {
        wgpu::Backends::VULKAN
    } else {
        wgpu::Backends::GL
    }
}

/// An adapter that can present to `surface`. On Android the instance holds one backend (see
/// [`instance_descriptor`]): its first adapter that supports the surface. Elsewhere: wgpu's
/// choice for the power preference.
pub async fn request_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    power_preference: wgpu::PowerPreference,
) -> Option<wgpu::Adapter> {
    if cfg!(target_os = "android") {
        for backends in [wgpu::Backends::VULKAN, wgpu::Backends::GL] {
            if let Some(adapter) = instance
                .enumerate_adapters(backends)
                .into_iter()
                .find(|a| a.is_surface_supported(surface))
            {
                return Some(adapter);
            }
        }
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
}

/// The highest of wgpu's limit tiers the adapter supports: `Limits::default()` on desktop GPUs
/// and recent phones, `downlevel_defaults()` on older mobile GPUs, `downlevel_webgl2_defaults()`
/// on OpenGL ES 3.0; texture sizes are raised to what the adapter allows. Requesting
/// `Limits::default()` outright fails `request_device` on GPUs below it.
pub fn supported_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    let supported = adapter.limits();
    for wanted in [
        wgpu::Limits::default(),
        wgpu::Limits::downlevel_defaults(),
        wgpu::Limits::downlevel_webgl2_defaults(),
    ] {
        let wanted = wanted.using_resolution(supported.clone());
        if wanted.check_limits(&supported) {
            return wanted;
        }
    }
    supported
}

/// A surface configuration for a window of `width`×`height`: an sRGB format when the surface
/// has one, vsync (`AutoVsync`, FIFO on phones), the surface's first alpha mode.
pub fn surface_config(
    capabilities: &wgpu::SurfaceCapabilities,
    width: u32,
    height: u32,
) -> Option<wgpu::SurfaceConfiguration> {
    let format = capabilities
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| capabilities.formats.first().copied())?;
    Some(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto),
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    })
}

/// A window's surface that can go away and come back (Android's suspend and resume).
///
/// Keep it where the `wgpu::Surface` used to be. Call [`SurfaceSlot::suspend`] from
/// `ApplicationHandler::suspended` and [`SurfaceSlot::resume`] from `resumed` (after the first);
/// while suspended, [`SurfaceSlot::get_current_texture`] answers `SurfaceError::Lost` and
/// `configure` only remembers the configuration, so the usual "lost: reconfigure and skip the
/// frame" path does the right thing.
pub struct SurfaceSlot {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl SurfaceSlot {
    /// Creates the window's surface on `instance` (keep the instance the adapter came from).
    pub fn new(
        instance: wgpu::Instance,
        window: Arc<Window>,
    ) -> Result<Self, wgpu::CreateSurfaceError> {
        let surface = instance.create_surface(window.clone())?;
        Ok(Self::from_surface(instance, window, surface))
    }

    /// Wraps a surface already made from `window` on `instance`.
    pub fn from_surface(
        instance: wgpu::Instance,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
    ) -> Self {
        Self {
            instance,
            window,
            surface: Some(surface),
            config: None,
        }
    }

    /// The instance the surface belongs to.
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// The window.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// The surface, while there is one.
    pub fn surface(&self) -> Option<&wgpu::Surface<'static>> {
        self.surface.as_ref()
    }

    /// Whether there is a surface to draw into.
    pub fn is_ready(&self) -> bool {
        self.surface.is_some()
    }

    /// The configuration last given to [`SurfaceSlot::configure`].
    pub fn config(&self) -> Option<&wgpu::SurfaceConfiguration> {
        self.config.as_ref()
    }

    /// The surface's capabilities with an adapter (empty while suspended).
    pub fn get_capabilities(&self, adapter: &wgpu::Adapter) -> wgpu::SurfaceCapabilities {
        self.surface
            .as_ref()
            .map(|s| s.get_capabilities(adapter))
            .unwrap_or_default()
    }

    /// Configures the surface (remembered for the next [`SurfaceSlot::resume`]). Sizes of zero
    /// are skipped, as wgpu would reject them.
    pub fn configure(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.config = Some(config.clone());
        if config.width == 0 || config.height == 0 {
            return;
        }
        if let Some(surface) = &self.surface {
            surface.configure(device, config);
        }
    }

    /// The next frame, or `SurfaceError::Lost` while suspended.
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        match &self.surface {
            Some(surface) => surface.get_current_texture(),
            None => Err(wgpu::SurfaceError::Lost),
        }
    }

    /// Drops the surface (call from `suspended`: Android destroys the window right after).
    pub fn suspend(&mut self) {
        self.surface = None;
    }

    /// Makes the surface again from the window and configures it like before, at the window's
    /// current size. Returns false when it already had one.
    pub fn resume(&mut self, device: &wgpu::Device) -> Result<bool, wgpu::CreateSurfaceError> {
        if self.surface.is_some() {
            return Ok(false);
        }
        let surface = self.instance.create_surface(self.window.clone())?;
        if let Some(config) = &mut self.config {
            let size = self.window.inner_size();
            if size.width > 0 && size.height > 0 {
                config.width = size.width;
                config.height = size.height;
                surface.configure(device, config);
            }
        }
        self.surface = Some(surface);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configs_prefer_srgb() {
        let caps = wgpu::SurfaceCapabilities {
            formats: vec![
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ],
            present_modes: vec![wgpu::PresentMode::Fifo],
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };
        let config = surface_config(&caps, 0, 720).unwrap();
        assert_eq!(config.format, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!((config.width, config.height), (1, 720));
        assert_eq!(config.alpha_mode, wgpu::CompositeAlphaMode::Opaque);
        assert!(surface_config(&wgpu::SurfaceCapabilities::default(), 8, 8).is_none());
    }

    #[test]
    fn the_descriptor_is_built_for_the_platform() {
        let descriptor = instance_descriptor();
        if cfg!(target_os = "android") {
            // One backend, never both: their two surfaces can't share the window.
            let vulkan = descriptor.backends.contains(wgpu::Backends::VULKAN);
            let gl = descriptor.backends.contains(wgpu::Backends::GL);
            assert!(vulkan != gl || std::env::var_os("WGPU_BACKEND").is_some());
        } else {
            assert!(!descriptor.backends.is_empty());
        }
    }
}
