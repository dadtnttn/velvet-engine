//! Multi-camera render plan structures.

use velvet_math::{Color, Rect, Vec2};

use crate::batch::SpriteBatch;
use crate::camera::Camera2D;
use crate::culling::CameraFrustum2D;
use crate::letterbox::Letterbox;

/// Target surface region for a camera (viewport in pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Origin x (pixels).
    pub x: f32,
    /// Origin y (pixels).
    pub y: f32,
    /// Width pixels.
    pub width: f32,
    /// Height pixels.
    pub height: f32,
}

impl Viewport {
    /// Full target.
    pub fn full(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    /// From letterbox drawable region.
    pub fn from_letterbox(lb: &Letterbox) -> Self {
        let o = lb.offset();
        let s = lb.size();
        Self {
            x: o.x,
            y: o.y,
            width: s.x,
            height: s.y,
        }
    }

    /// As rect.
    pub fn as_rect(self) -> Rect {
        Rect::from_pos_size(
            Vec2::new(self.x, self.y),
            Vec2::new(self.width, self.height),
        )
    }

    /// Aspect ratio.
    pub fn aspect(self) -> f32 {
        self.width / self.height.max(1e-6)
    }
}

/// Clear mode for a camera pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClearMode {
    /// Do not clear (UI overlay).
    None,
    /// Clear color only.
    Color(Color),
    /// Clear color + depth if available.
    ColorDepth(Color),
}

impl Default for ClearMode {
    fn default() -> Self {
        Self::Color(Color::rgb(0.08, 0.07, 0.12))
    }
}

/// One camera pass in the frame plan.
#[derive(Debug)]
pub struct CameraPass {
    /// Debug name.
    pub name: String,
    /// Camera state.
    pub camera: Camera2D,
    /// Pixel viewport.
    pub viewport: Viewport,
    /// Clear mode.
    pub clear: ClearMode,
    /// Sort / draw priority (higher later).
    pub priority: i32,
    /// Whether enabled.
    pub enabled: bool,
    /// Optional layer mask (bitfield for game use).
    pub layer_mask: u32,
    /// Frustum for culling (cached).
    pub frustum: CameraFrustum2D,
    /// Per-pass sprite batch.
    pub batch: SpriteBatch,
}

impl CameraPass {
    /// Create a pass from camera + viewport.
    pub fn new(name: impl Into<String>, camera: Camera2D, viewport: Viewport) -> Self {
        let clear = camera
            .clear
            .map(|c| ClearMode::Color(Color::rgba(c[0], c[1], c[2], c[3])))
            .unwrap_or(ClearMode::None);
        let priority = camera.priority;
        let frustum = CameraFrustum2D::from_camera(&camera);
        Self {
            name: name.into(),
            camera,
            viewport,
            clear,
            priority,
            enabled: true,
            layer_mask: u32::MAX,
            frustum,
            batch: SpriteBatch::new(),
        }
    }

    /// Rebuild frustum from camera.
    pub fn sync_frustum(&mut self) {
        self.frustum = CameraFrustum2D::from_camera(&self.camera);
    }

    /// Clear batch for new frame.
    pub fn begin_frame(&mut self) {
        self.batch.clear();
        self.sync_frustum();
    }
}

/// Full multi-camera render plan for one frame.
#[derive(Debug, Default)]
pub struct RenderPlan {
    /// Ordered camera passes (sorted by priority when finalized).
    pub passes: Vec<CameraPass>,
    /// Target surface size in pixels.
    pub surface_size: (u32, u32),
    /// Whether plan has been sorted this frame.
    sorted: bool,
}

impl RenderPlan {
    /// Empty plan.
    pub fn new(surface_width: u32, surface_height: u32) -> Self {
        Self {
            passes: Vec::new(),
            surface_size: (surface_width, surface_height),
            sorted: false,
        }
    }

    /// Add a camera pass.
    pub fn add_pass(&mut self, pass: CameraPass) {
        self.passes.push(pass);
        self.sorted = false;
    }

    /// Add default full-screen camera.
    pub fn add_main_camera(&mut self, camera: Camera2D) {
        let vp = Viewport::full(self.surface_size.0 as f32, self.surface_size.1 as f32);
        self.add_pass(CameraPass::new("main", camera, vp));
    }

    /// Number of passes.
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    /// Sort passes by priority ascending (low first).
    pub fn finalize_order(&mut self) {
        self.passes.sort_by_key(|p| p.priority);
        self.sorted = true;
    }

    /// Whether sorted.
    pub fn is_sorted(&self) -> bool {
        self.sorted
    }

    /// Begin frame: clear batches, resync frustums.
    pub fn begin_frame(&mut self) {
        for p in &mut self.passes {
            p.begin_frame();
        }
        self.sorted = false;
    }

    /// Enabled passes in order (finalizes if needed).
    pub fn enabled_passes(&mut self) -> impl Iterator<Item = &mut CameraPass> {
        if !self.sorted {
            self.finalize_order();
        }
        self.passes
            .iter_mut()
            .filter(|p| p.enabled && p.camera.enabled)
    }

    /// Total sprites across all batches.
    pub fn total_sprites(&self) -> usize {
        self.passes.iter().map(|p| p.batch.len()).sum()
    }

    /// Estimate total draw calls after sorting each batch.
    pub fn estimate_draw_calls(&mut self) -> u32 {
        let mut total = 0;
        for p in &mut self.passes {
            p.batch.sort();
            total += p.batch.estimate_draw_calls();
        }
        total
    }

    /// Find pass by name.
    pub fn pass_mut(&mut self, name: &str) -> Option<&mut CameraPass> {
        self.passes.iter_mut().find(|p| p.name == name)
    }

    /// Split-screen two horizontal cameras.
    pub fn split_horizontal(
        surface_w: u32,
        surface_h: u32,
        top: Camera2D,
        bottom: Camera2D,
    ) -> Self {
        let mut plan = Self::new(surface_w, surface_h);
        let h = surface_h as f32 * 0.5;
        let w = surface_w as f32;
        plan.add_pass(CameraPass::new(
            "top",
            top,
            Viewport {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            },
        ));
        plan.add_pass(CameraPass::new(
            "bottom",
            bottom,
            Viewport {
                x: 0.0,
                y: h,
                width: w,
                height: h,
            },
        ));
        plan.finalize_order();
        plan
    }

    /// Split-screen two vertical cameras.
    pub fn split_vertical(surface_w: u32, surface_h: u32, left: Camera2D, right: Camera2D) -> Self {
        let mut plan = Self::new(surface_w, surface_h);
        let w = surface_w as f32 * 0.5;
        let h = surface_h as f32;
        plan.add_pass(CameraPass::new(
            "left",
            left,
            Viewport {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            },
        ));
        plan.add_pass(CameraPass::new(
            "right",
            right,
            Viewport {
                x: w,
                y: 0.0,
                width: w,
                height: h,
            },
        ));
        plan.finalize_order();
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::TextureId;
    use velvet_math::Transform2D;

    #[test]
    fn priority_order() {
        let mut plan = RenderPlan::new(800, 600);
        let mut ui_cam = Camera2D::virtual_res(800.0, 600.0);
        ui_cam.priority = 10;
        let mut world = Camera2D::virtual_res(800.0, 600.0);
        world.priority = 0;
        plan.add_pass(CameraPass::new("ui", ui_cam, Viewport::full(800.0, 600.0)));
        plan.add_pass(CameraPass::new(
            "world",
            world,
            Viewport::full(800.0, 600.0),
        ));
        plan.finalize_order();
        assert_eq!(plan.passes[0].name, "world");
        assert_eq!(plan.passes[1].name, "ui");
    }

    #[test]
    fn split_horizontal_viewports() {
        let plan = RenderPlan::split_horizontal(100, 100, Camera2D::default(), Camera2D::default());
        assert_eq!(plan.len(), 2);
        assert!((plan.passes[0].viewport.height - 50.0).abs() < 1e-3);
    }

    #[test]
    fn batch_stats() {
        let mut plan = RenderPlan::new(64, 64);
        plan.add_main_camera(Camera2D::default());
        let t = TextureId::allocate();
        plan.pass_mut("main").unwrap().batch.push_colored_quad(
            t,
            Transform2D::IDENTITY,
            Vec2::ONE,
            Color::WHITE,
            0.0,
        );
        assert_eq!(plan.total_sprites(), 1);
        assert!(plan.estimate_draw_calls() >= 1);
    }
}
