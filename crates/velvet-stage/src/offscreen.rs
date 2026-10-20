//! Drawing a stage without a window: for tests, bots and captures.

use crate::render::StageRenderer;
use crate::stage::Stage;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// A graphics device of its own, drawing stages into pictures.
pub struct Offscreen {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: StageRenderer,
}

impl Offscreen {
    /// Opens a graphics device without a window (4 samples a pixel).
    pub fn new() -> Result<Offscreen, String> {
        let mut descriptor = wgpu::InstanceDescriptor::default();
        if cfg!(target_os = "windows") {
            descriptor.backends = wgpu::Backends::DX12;
        }
        let instance = wgpu::Instance::new(&descriptor.with_env());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or("no graphics adapter")?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor { label: Some("velvet-stage offscreen"), required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default(), memory_hints: wgpu::MemoryHints::MemoryUsage },
            None,
        ))
        .map_err(|e| format!("the graphics device: {e}"))?;
        let renderer = StageRenderer::new(&device, &queue, FORMAT, 4);
        Ok(Offscreen { device, queue, renderer })
    }

    /// The stage drawn at `width`×`height` pixels: sRGB RGBA, row after row.
    pub fn render(&mut self, stage: &Stage, width: u32, height: u32) -> Vec<u8> {
        render_rgba(&self.device, &self.queue, &mut self.renderer, stage, width, height)
    }

    /// The stage drawn and saved as a PNG.
    pub fn save_png(&mut self, stage: &Stage, width: u32, height: u32, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let pixels = self.render(stage, width, height);
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8).map_err(|e| e.to_string())
    }
}

/// Draws a stage into a texture of its own and reads it back (also used for screenshots of a window).
pub fn render_rgba(device: &wgpu::Device, queue: &wgpu::Queue, renderer: &mut StageRenderer, stage: &Stage, width: u32, height: u32) -> Vec<u8> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-stage capture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("velvet-stage capture") });
    renderer.render(device, queue, &mut encoder, &view, FORMAT, (width, height), [0.0, 0.0, width as f32, height as f32], stage);
    let row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("velvet-stage capture"),
        size: (row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let start = (y * row) as usize;
        out.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();
    out
}
