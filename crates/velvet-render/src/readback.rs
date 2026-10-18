//! Reading what was rendered back to the CPU: a frame for a capture, a still for a menu.
use crate::Render3DError;

/// Reads the whole of a 2D colour texture of 8 bits per channel (RGBA or BGRA) back as RGBA8,
/// row after row from the top, waiting for the GPU. The texture needs `COPY_SRC`.
pub fn read_texture_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
) -> Result<Vec<u8>, Render3DError> {
    let (width, height) = (texture.width(), texture.height());
    let row = width * 4;
    let padded =
        row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("velvet-readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
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
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    let _ = device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|e| Render3DError::Gpu(format!("readback: {e}")))?
        .map_err(|e| Render3DError::Gpu(format!("readback: {e}")))?;
    let mut pixels = unpad_rows(
        &buffer.slice(..).get_mapped_range(),
        padded as usize,
        row as usize,
    );
    buffer.unmap();
    if matches!(
        texture.format(),
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
    Ok(pixels)
}

/// Rows of `row` bytes out of a copy whose rows are `padded` bytes apart.
fn unpad_rows(data: &[u8], padded: usize, row: usize) -> Vec<u8> {
    if padded == row {
        return data.to_vec();
    }
    data.chunks_exact(padded)
        .flat_map(|r| &r[..row])
        .copied()
        .collect()
}

/// Whether an RGBA8 frame shows nothing: no pixel lit above near-black in red and green (a
/// capture of a frame that was never drawn).
pub fn frame_is_blank(rgba: &[u8]) -> bool {
    !rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[0] > 10 && p[1] > 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_rows_are_cut_back_to_the_image() {
        let data: Vec<u8> = (0..2 * 8).map(|i| i as u8).collect();
        assert_eq!(
            unpad_rows(&data, 8, 6),
            vec![0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13]
        );
        assert_eq!(unpad_rows(&data, 8, 8), data);
    }

    #[test]
    fn a_black_or_blue_frame_is_blank_and_a_lit_one_is_not() {
        assert!(frame_is_blank(&[0, 0, 0, 255, 8, 9, 10, 255]));
        assert!(
            frame_is_blank(&[0, 0, 200, 255]),
            "only red and green count"
        );
        assert!(!frame_is_blank(&[0, 0, 0, 255, 40, 30, 20, 255]));
    }
}
