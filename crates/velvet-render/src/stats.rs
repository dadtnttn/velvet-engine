//! Per-frame render statistics.

use serde::{Deserialize, Serialize};

/// GPU/CPU render counters for diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderStats {
    /// Sprites submitted this frame.
    pub sprites_submitted: u32,
    /// Draw calls issued.
    pub draw_calls: u32,
    /// Texture binds.
    pub texture_binds: u32,
    /// Cameras rendered.
    pub cameras: u32,
    /// Triangles (2 per sprite quad).
    pub triangles: u32,
    /// Live particles simulated / submitted.
    pub particles: u32,
    /// Debug overlay shapes.
    pub debug_shapes: u32,
    /// Debug overlay text lines.
    pub debug_lines: u32,
    /// GPU frame time microseconds (if measured).
    pub gpu_time_us: u64,
    /// CPU encode time microseconds.
    pub cpu_encode_us: u64,
}

impl RenderStats {
    /// Reset counters.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Finalize derived fields after draw.
    pub fn finish_draw_calls(&mut self, draw_calls: u32) {
        self.draw_calls = draw_calls;
        self.triangles = self
            .sprites_submitted
            .saturating_add(self.particles)
            .saturating_mul(2);
    }

    /// Record particle batch size.
    pub fn record_particles(&mut self, count: u32) {
        self.particles = self.particles.saturating_add(count);
    }

    /// Record debug overlay counts.
    pub fn record_debug(&mut self, shapes: u32, lines: u32) {
        self.debug_shapes = self.debug_shapes.saturating_add(shapes);
        self.debug_lines = self.debug_lines.saturating_add(lines);
    }
}
