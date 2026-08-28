//! Software-frame presentation for CPU-rendered games.
//!
//! The game keeps a stable virtual canvas while this presenter maps it into a
//! resizable physical window. Letterbox bars and pointer conversion use the
//! same integer viewport, so rendering and interaction cannot drift apart.

/// Sampling used while scaling a software framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoftwareScalingFilter {
    /// Fast pixel-preserving sampling.
    Nearest,
    /// Smooth sampling suited to illustrated games and visual novels.
    #[default]
    Bilinear,
}

/// Exact integer viewport occupied by a virtual canvas in a physical buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoftwareViewport {
    /// Left edge in physical pixels.
    pub x: u32,
    /// Top edge in physical pixels.
    pub y: u32,
    /// Drawn width in physical pixels.
    pub width: u32,
    /// Drawn height in physical pixels.
    pub height: u32,
    /// Physical framebuffer width.
    pub physical_width: u32,
    /// Physical framebuffer height.
    pub physical_height: u32,
    /// Virtual canvas width.
    pub virtual_width: u32,
    /// Virtual canvas height.
    pub virtual_height: u32,
}

impl SoftwareViewport {
    /// Compute a centered, aspect-preserving viewport.
    pub fn letterbox(
        physical_width: u32,
        physical_height: u32,
        virtual_width: u32,
        virtual_height: u32,
    ) -> Option<Self> {
        if physical_width == 0 || physical_height == 0 || virtual_width == 0 || virtual_height == 0
        {
            return None;
        }

        let scale = (physical_width as f64 / virtual_width as f64)
            .min(physical_height as f64 / virtual_height as f64);
        let width = ((virtual_width as f64 * scale).round() as u32).clamp(1, physical_width);
        let height = ((virtual_height as f64 * scale).round() as u32).clamp(1, physical_height);

        Some(Self {
            x: (physical_width - width) / 2,
            y: (physical_height - height) / 2,
            width,
            height,
            physical_width,
            physical_height,
            virtual_width,
            virtual_height,
        })
    }

    /// Convert a physical pointer position into virtual canvas coordinates.
    ///
    /// Returns `None` while the pointer is inside a letterbox bar.
    pub fn physical_to_virtual(self, physical_x: f64, physical_y: f64) -> Option<(f32, f32)> {
        if !physical_x.is_finite() || !physical_y.is_finite() {
            return None;
        }
        let local_x = physical_x - f64::from(self.x);
        let local_y = physical_y - f64::from(self.y);
        if local_x < 0.0
            || local_y < 0.0
            || local_x >= f64::from(self.width)
            || local_y >= f64::from(self.height)
        {
            return None;
        }

        let virtual_x = local_x * f64::from(self.virtual_width) / f64::from(self.width);
        let virtual_y = local_y * f64::from(self.virtual_height) / f64::from(self.height);
        Some((virtual_x as f32, virtual_y as f32))
    }

    /// Convert virtual canvas coordinates into physical window coordinates.
    pub fn virtual_to_physical(self, virtual_x: f32, virtual_y: f32) -> Option<(f32, f32)> {
        if !virtual_x.is_finite()
            || !virtual_y.is_finite()
            || virtual_x < 0.0
            || virtual_y < 0.0
            || virtual_x > self.virtual_width as f32
            || virtual_y > self.virtual_height as f32
        {
            return None;
        }
        Some((
            self.x as f32 + virtual_x * self.width as f32 / self.virtual_width as f32,
            self.y as f32 + virtual_y * self.height as f32 / self.virtual_height as f32,
        ))
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct AxisSample {
    low: u32,
    high: u32,
    weight: u16,
}

/// Cached software-frame scaler for resizable windows.
///
/// Sampling tables rebuild only after a resize, avoiding per-frame allocation
/// and repeated coordinate calculations during animation.
#[derive(Debug, Default)]
pub struct SoftwarePresenter {
    source_size: (u32, u32),
    target_size: (u32, u32),
    viewport: Option<SoftwareViewport>,
    x_samples: Vec<AxisSample>,
    y_samples: Vec<AxisSample>,
}

impl SoftwarePresenter {
    /// Create an empty presenter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scale a virtual XRGB framebuffer into a physical XRGB framebuffer.
    ///
    /// `bar_color` fills the area outside the aspect-preserving viewport.
    /// Returns the viewport used, or `None` for zero dimensions or undersized
    /// buffers.
    #[allow(clippy::too_many_arguments)]
    pub fn present(
        &mut self,
        source: &[u32],
        source_width: u32,
        source_height: u32,
        target: &mut [u32],
        target_width: u32,
        target_height: u32,
        bar_color: u32,
        filter: SoftwareScalingFilter,
    ) -> Option<SoftwareViewport> {
        let source_len = pixel_count(source_width, source_height)?;
        let target_len = pixel_count(target_width, target_height)?;
        if source.len() < source_len || target.len() < target_len {
            return None;
        }

        let viewport =
            SoftwareViewport::letterbox(target_width, target_height, source_width, source_height)?;

        if source_width == target_width && source_height == target_height {
            target[..target_len].copy_from_slice(&source[..source_len]);
            self.source_size = (source_width, source_height);
            self.target_size = (target_width, target_height);
            self.viewport = Some(viewport);
            self.x_samples.clear();
            self.y_samples.clear();
            return Some(viewport);
        }

        self.ensure_sampling_tables(viewport);
        target[..target_len].fill(bar_color);

        match filter {
            SoftwareScalingFilter::Nearest => self.present_nearest(source, target, viewport),
            SoftwareScalingFilter::Bilinear => self.present_bilinear(source, target, viewport),
        }
        Some(viewport)
    }

    fn ensure_sampling_tables(&mut self, viewport: SoftwareViewport) {
        let source_size = (viewport.virtual_width, viewport.virtual_height);
        let target_size = (viewport.physical_width, viewport.physical_height);
        if self.source_size == source_size
            && self.target_size == target_size
            && self.viewport == Some(viewport)
        {
            return;
        }

        self.source_size = source_size;
        self.target_size = target_size;
        self.viewport = Some(viewport);
        self.x_samples = (0..viewport.width)
            .map(|index| axis_sample(index, viewport.width, viewport.virtual_width))
            .collect();
        self.y_samples = (0..viewport.height)
            .map(|index| axis_sample(index, viewport.height, viewport.virtual_height))
            .collect();
    }

    fn present_nearest(&self, source: &[u32], target: &mut [u32], viewport: SoftwareViewport) {
        for (target_y, sample_y) in self.y_samples.iter().enumerate() {
            let source_y = if sample_y.weight >= 128 {
                sample_y.high
            } else {
                sample_y.low
            };
            let source_row = source_y as usize * viewport.virtual_width as usize;
            let target_row = (viewport.y as usize + target_y) * viewport.physical_width as usize
                + viewport.x as usize;

            for (target_x, sample_x) in self.x_samples.iter().enumerate() {
                let source_x = if sample_x.weight >= 128 {
                    sample_x.high
                } else {
                    sample_x.low
                };
                target[target_row + target_x] = source[source_row + source_x as usize];
            }
        }
    }

    fn present_bilinear(&self, source: &[u32], target: &mut [u32], viewport: SoftwareViewport) {
        let source_width = viewport.virtual_width as usize;
        let target_width = viewport.physical_width as usize;
        for (target_y, sample_y) in self.y_samples.iter().enumerate() {
            let row_low = sample_y.low as usize * source_width;
            let row_high = sample_y.high as usize * source_width;
            let target_row = (viewport.y as usize + target_y) * target_width + viewport.x as usize;

            for (target_x, sample_x) in self.x_samples.iter().enumerate() {
                let low = sample_x.low as usize;
                let high = sample_x.high as usize;
                target[target_row + target_x] = bilinear_xrgb(
                    source[row_low + low],
                    source[row_low + high],
                    source[row_high + low],
                    source[row_high + high],
                    sample_x.weight,
                    sample_y.weight,
                );
            }
        }
    }
}

fn pixel_count(width: u32, height: u32) -> Option<usize> {
    usize::try_from(u64::from(width).checked_mul(u64::from(height))?).ok()
}

fn axis_sample(index: u32, output_len: u32, source_len: u32) -> AxisSample {
    let position = ((f64::from(index) + 0.5) * f64::from(source_len) / f64::from(output_len) - 0.5)
        .clamp(0.0, f64::from(source_len - 1));
    let low = position.floor() as u32;
    let high = (low + 1).min(source_len - 1);
    let weight = ((position - f64::from(low)) * 256.0).round() as u16;
    AxisSample {
        low,
        high,
        weight: weight.min(256),
    }
}

fn lerp_channel(low: u32, high: u32, weight: u16) -> u32 {
    let weight = u32::from(weight);
    (low * (256 - weight) + high * weight + 128) >> 8
}

fn bilinear_xrgb(c00: u32, c10: u32, c01: u32, c11: u32, wx: u16, wy: u16) -> u32 {
    let channel = |shift: u32| {
        let top = lerp_channel((c00 >> shift) & 0xff, (c10 >> shift) & 0xff, wx);
        let bottom = lerp_channel((c01 >> shift) & 0xff, (c11 >> shift) & 0xff, wx);
        lerp_channel(top, bottom, wy)
    };
    0xff00_0000 | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_centers_widescreen_inside_four_by_three() {
        let viewport = SoftwareViewport::letterbox(800, 600, 1920, 1080).expect("viewport");
        assert_eq!((viewport.x, viewport.y), (0, 75));
        assert_eq!((viewport.width, viewport.height), (800, 450));
        assert_eq!(
            viewport.physical_to_virtual(400.0, 300.0),
            Some((960.0, 540.0))
        );
        assert_eq!(viewport.physical_to_virtual(400.0, 20.0), None);
    }

    #[test]
    fn presenter_fills_bars_and_draws_scaled_content() {
        let source = [0xffff_0000, 0xff00_00ff];
        let mut target = [0u32; 16];
        let mut presenter = SoftwarePresenter::new();
        let viewport = presenter
            .present(
                &source,
                2,
                1,
                &mut target,
                4,
                4,
                0xff01_0203,
                SoftwareScalingFilter::Bilinear,
            )
            .expect("present");

        assert_eq!(
            (viewport.x, viewport.y, viewport.width, viewport.height),
            (0, 1, 4, 2)
        );
        assert!(target[..4].iter().all(|pixel| *pixel == 0xff01_0203));
        assert!(target[12..].iter().all(|pixel| *pixel == 0xff01_0203));
        assert_ne!(target[4], 0xff01_0203);
        assert_ne!(target[7], 0xff01_0203);
    }

    #[test]
    fn native_size_uses_exact_copy() {
        let source = [1, 2, 3, 4];
        let mut target = [0; 4];
        let mut presenter = SoftwarePresenter::new();
        presenter
            .present(
                &source,
                2,
                2,
                &mut target,
                2,
                2,
                0,
                SoftwareScalingFilter::Nearest,
            )
            .expect("present");
        assert_eq!(target, source);
    }
}
