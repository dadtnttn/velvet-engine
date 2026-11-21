//! Drawing a stage without a window: for tests, bots and captures.

use std::cell::RefCell;
use std::rc::Rc;

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
            &wgpu::DeviceDescriptor {
                label: Some("velvet-stage offscreen"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        ))
        .map_err(|e| format!("the graphics device: {e}"))?;
        // VELVET_FRUGAL=1: the renderer a frugal window uses, to check that its picture is the same.
        let make = if std::env::var_os("VELVET_FRUGAL").is_some() {
            StageRenderer::frugal
        } else {
            StageRenderer::new
        };
        let renderer = make(&device, &queue, FORMAT, 4);
        Ok(Offscreen {
            device,
            queue,
            renderer,
        })
    }

    /// The stage drawn at `width`×`height` pixels: sRGB RGBA, row after row.
    /// Returns an empty vector when the capture cannot be read.
    pub fn render(&mut self, stage: &Stage, width: u32, height: u32) -> Vec<u8> {
        render_rgba(
            &self.device,
            &self.queue,
            &mut self.renderer,
            stage,
            width,
            height,
        )
    }

    /// The stage drawn and saved as a PNG.
    pub fn save_png(
        &mut self,
        stage: &Stage,
        width: u32,
        height: u32,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), String> {
        let pixels = render_rgba_as(
            &self.device,
            &self.queue,
            &mut self.renderer,
            FORMAT,
            stage,
            width,
            height,
        )?;
        image::save_buffer(path, &pixels, width, height, image::ColorType::Rgba8)
            .map_err(|e| e.to_string())
    }
}

/// The renderer a window draws with, for drawing a stage elsewhere too (a save's picture) with the same device and the
/// textures already there. Stays on the thread of the window.
#[derive(Clone)]
pub struct SharedRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderer: Rc<RefCell<StageRenderer>>,
    /// The format the renderer draws in.
    pub format: wgpu::TextureFormat,
}

impl SharedRenderer {
    /// The stage drawn at `width`×`height` pixels: sRGB RGBA, row after row.
    /// Returns an empty vector when the capture cannot be read.
    /// Returns an empty vector when the capture cannot be read.
    pub fn render_rgba(&self, stage: &Stage, width: u32, height: u32) -> Vec<u8> {
        self.try_render_rgba(stage, width, height)
            .unwrap_or_default()
    }

    /// Reads the stage back, reporting unsupported dimensions and GPU readback failures.
    pub fn try_render_rgba(
        &self,
        stage: &Stage,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        render_rgba_as(
            &self.device,
            &self.queue,
            &mut self.renderer.borrow_mut(),
            self.format,
            stage,
            width,
            height,
        )
    }
}

/// Draws a stage into a texture of its own and reads it back (for a renderer made for [`FORMAT`]).
/// Returns an empty vector when the capture cannot be read.
pub fn render_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut StageRenderer,
    stage: &Stage,
    width: u32,
    height: u32,
) -> Vec<u8> {
    render_rgba_as(device, queue, renderer, FORMAT, stage, width, height).unwrap_or_default()
}

fn readback_layout(limits: &wgpu::Limits, width: u32, height: u32) -> Result<(u32, u64), String> {
    if width == 0 || height == 0 || width.max(height) > limits.max_texture_dimension_2d {
        return Err(format!("unsupported stage capture size: {width}x{height}"));
    }
    let alignment = u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let row = (u64::from(width) * 4).div_ceil(alignment) * alignment;
    let size = row
        .checked_mul(u64::from(height))
        .filter(|size| *size <= limits.max_buffer_size && usize::try_from(*size).is_ok())
        .ok_or("stage capture exceeds the graphics readback buffer limit")?;
    let row = u32::try_from(row).map_err(|_| "stage capture row is too wide")?;
    Ok((row, size))
}

/// Draws a stage into a texture of `format` (the renderer's) and reads it back as RGBA.
fn render_rgba_as(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut StageRenderer,
    format: wgpu::TextureFormat,
    stage: &Stage,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let (row, size) = readback_layout(&device.limits(), width, height)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-stage capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("velvet-stage capture"),
    });
    renderer.render(
        device,
        queue,
        &mut encoder,
        &view,
        format,
        (width, height),
        [0.0, 0.0, width as f32, height as f32],
        stage,
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("velvet-stage capture"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    let (send, mapped) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = send.send(result);
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    mapped
        .try_recv()
        .map_err(|e| format!("stage capture did not complete: {e}"))?
        .map_err(|e| format!("stage capture readback failed: {e}"))?;
    let data = slice.get_mapped_range();
    let width_bytes = width as usize * 4;
    let mut out = Vec::with_capacity(width_bytes * height as usize);
    for line in data.chunks_exact(row as usize) {
        out.extend_from_slice(&line[..width_bytes]);
    }
    drop(data);
    buffer.unmap();
    if matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for i in (0..out.len()).step_by(4) {
            out.swap(i, i + 2);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readback_rejects_invalid_texture_and_buffer_sizes() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 128,
            max_buffer_size: 1024,
            ..wgpu::Limits::default()
        };
        assert!(readback_layout(&limits, 0, 1).is_err());
        assert!(readback_layout(&limits, 129, 1).is_err());
        assert!(readback_layout(&limits, 128, 3).is_err());
        assert_eq!(readback_layout(&limits, 65, 2).unwrap(), (512, 1024));
    }
}
