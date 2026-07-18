//! CPU color buffer from cellular world — bridge for any renderer (wgpu/UI).

use crate::chunk::{ChunkCoord, CHUNK_SIZE};
use crate::world::World;

/// RGBA8 image buffer.
#[derive(Debug, Clone)]
pub struct ColorBuffer {
    /// Width in pixels/cells.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Row-major RGBA.
    pub pixels: Vec<u8>,
}

impl ColorBuffer {
    /// Empty transparent buffer.
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            width,
            height,
            pixels: vec![0; n],
        }
    }

    /// Set pixel.
    pub fn set_rgba(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        self.pixels[i] = r;
        self.pixels[i + 1] = g;
        self.pixels[i + 2] = b;
        self.pixels[i + 3] = a;
    }
}

/// Render a world window into a color buffer.
///
/// `origin_x/y` = world cell at buffer (0,0) bottom-left.
/// Buffer y increases upward in world space (flip if your GPU wants top-left).
pub fn render_world_window(
    world: &World,
    origin_x: i32,
    origin_y: i32,
    width: u32,
    height: u32,
) -> ColorBuffer {
    let mut buf = ColorBuffer::new(width, height);
    for py in 0..height {
        for px in 0..width {
            let wx = origin_x + px as i32;
            let wy = origin_y + py as i32;
            let cell = world.get(wx, wy);
            if cell.is_air() {
                continue;
            }
            let def = world.materials.get(cell.material);
            let mut rgba = def.color;
            // temperature tint
            if cell.temp > 100.0 {
                let t = ((cell.temp - 100.0) / 400.0).clamp(0.0, 1.0);
                rgba[0] = rgba[0].saturating_add((t * 80.0) as u8);
                rgba[2] = rgba[2].saturating_sub((t * 40.0) as u8);
            }
            // burning
            if cell.flags.contains(crate::cell::CellFlags::BURNING) {
                rgba = [255, 120, 40, 255];
            }
            // meta variance
            if def.color_variance > 0 {
                let v = def.color_variance;
                let j = ((wx.wrapping_mul(374761393) ^ wy.wrapping_mul(668265263)) as u32) % (v as u32 * 2 + 1);
                let d = j as i16 - v as i16;
                for k in 0..3 {
                    rgba[k] = (rgba[k] as i16 + d).clamp(0, 255) as u8;
                }
            }
            buf.set_rgba(px, py, rgba[0], rgba[1], rgba[2], rgba[3]);
        }
    }
    buf
}

/// Render a single chunk into a CHUNK_SIZE² buffer.
pub fn render_chunk(world: &World, coord: ChunkCoord) -> ColorBuffer {
    let (ox, oy) = coord.origin_cell();
    render_world_window(world, ox, oy, CHUNK_SIZE as u32, CHUNK_SIZE as u32)
}

/// Non-zero pixel count (for tests).
pub fn opaque_pixel_count(buf: &ColorBuffer) -> usize {
    buf.pixels.chunks(4).filter(|c| c[3] > 0).count()
}
