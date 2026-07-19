//! Product presenters: hybrid softbuffer (menu) + wgpu (game) path.
//!
//! Presentation **state** and paint/descriptor building live here so unit tests
//! never need a GPU. Hosts (demos, CLI) choose the active backend and submit
//! either CPU pixels or GPU quads.

use crate::product::VnSession;
use crate::product_paint::{
    paint_product_session, paint_to_render_descriptors, ProductPaintList, RenderDrawDescriptor,
};
use crate::product_raster::rasterize_product_paint;

/// Where product frames are presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PresenterBackend {
    /// CPU bitmap → softbuffer (title menu / fallback).
    #[default]
    Softbuffer,
    /// GPU path: paint descriptors → `velvet-render` sprite batch / wgpu.
    Wgpu,
}

impl PresenterBackend {
    /// Display name for logs / titles.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Softbuffer => "softbuffer",
            Self::Wgpu => "wgpu",
        }
    }
}

/// Screen phase for hybrid policy (menu may stay CPU; play prefers GPU).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PresenterPhase {
    /// Title / literary menu.
    #[default]
    Title,
    /// In-game VN / play surface.
    Play,
}

/// Hybrid product presenter: builds paint + GPU descriptors; tracks backend.
///
/// Does **not** own a GPU device — that stays in the host / `velvet-render`.
#[derive(Debug, Clone)]
pub struct ProductPresenter {
    /// Preferred backend when the phase allows GPU.
    preferred: PresenterBackend,
    /// Last resolved active backend (after hybrid policy).
    active: PresenterBackend,
    /// Current phase (title vs play).
    phase: PresenterPhase,
    /// Last paint list (CPU/GPU shared IR).
    last_paint: Option<ProductPaintList>,
    /// Last GPU-oriented draw descriptors.
    last_descs: Vec<RenderDrawDescriptor>,
    /// Whether GPU was requested but host fell back (honest env).
    gpu_fallback: bool,
    /// Optional fallback reason (no adapter, headless, etc.).
    fallback_reason: Option<String>,
}

impl Default for ProductPresenter {
    fn default() -> Self {
        Self::new(PresenterBackend::Wgpu)
    }
}

impl ProductPresenter {
    /// Create with a preferred backend (hybrid policy still applies).
    pub fn new(preferred: PresenterBackend) -> Self {
        Self {
            preferred,
            active: PresenterBackend::Softbuffer,
            phase: PresenterPhase::Title,
            last_paint: None,
            last_descs: Vec::new(),
            gpu_fallback: false,
            fallback_reason: None,
        }
    }

    /// Hybrid default: softbuffer menu, wgpu play.
    pub fn hybrid() -> Self {
        Self::new(PresenterBackend::Wgpu)
    }

    /// Preferred backend.
    pub fn preferred(&self) -> PresenterBackend {
        self.preferred
    }

    /// Active backend after policy / fallback.
    pub fn active(&self) -> PresenterBackend {
        self.active
    }

    /// Current phase.
    pub fn phase(&self) -> PresenterPhase {
        self.phase
    }

    /// Last paint list, if any.
    pub fn last_paint(&self) -> Option<&ProductPaintList> {
        self.last_paint.as_ref()
    }

    /// Last render descriptors (GPU batch input).
    pub fn last_descriptors(&self) -> &[RenderDrawDescriptor] {
        &self.last_descs
    }

    /// True when host reported GPU unavailable and we use softbuffer.
    pub fn gpu_fallback(&self) -> bool {
        self.gpu_fallback
    }

    /// Fallback reason string.
    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    /// Enter title phase (menu → softbuffer even if preferred is wgpu).
    pub fn set_phase_title(&mut self) {
        self.phase = PresenterPhase::Title;
        self.resolve_active(true);
    }

    /// Enter play phase (prefer wgpu when requested).
    pub fn set_phase_play(&mut self) {
        self.phase = PresenterPhase::Play;
        self.resolve_active(true);
    }

    /// Host reports GPU availability. When `available` is false, play falls back.
    pub fn set_gpu_available(&mut self, available: bool, reason: Option<impl Into<String>>) {
        if available {
            self.gpu_fallback = false;
            self.fallback_reason = None;
        } else {
            self.gpu_fallback = true;
            self.fallback_reason = reason.map(|r| r.into());
        }
        self.resolve_active(true);
    }

    /// Force preferred backend (does not ignore hybrid title→softbuffer).
    pub fn set_preferred(&mut self, backend: PresenterBackend) {
        self.preferred = backend;
        self.resolve_active(true);
    }

    fn resolve_active(&mut self, _recompute: bool) {
        self.active = match self.phase {
            // Hybrid: menu always softbuffer (OK per plan).
            PresenterPhase::Title => PresenterBackend::Softbuffer,
            PresenterPhase::Play => {
                if self.preferred == PresenterBackend::Wgpu && !self.gpu_fallback {
                    PresenterBackend::Wgpu
                } else {
                    PresenterBackend::Softbuffer
                }
            }
        };
    }

    /// Build paint list + descriptors from a live session (unit-testable, no GPU).
    pub fn present_session(&mut self, session: &VnSession) -> &ProductPaintList {
        let list = paint_product_session(session);
        self.last_descs = paint_to_render_descriptors(&list);
        self.last_paint = Some(list);
        self.last_paint.as_ref().expect("just set")
    }

    /// Only positive-size **quad** descriptors (safe GPU sprite batch input).
    pub fn gpu_quad_descriptors(&self) -> Vec<&RenderDrawDescriptor> {
        self.last_descs
            .iter()
            .filter(|d| d.kind == "quad" && d.w > 0.0 && d.h > 0.0)
            .collect()
    }

    /// Only **text** descriptors (for glyphon / velvet-text GPU path).
    pub fn gpu_text_descriptors(&self) -> Vec<&RenderDrawDescriptor> {
        self.last_descs
            .iter()
            .filter(|d| d.kind == "text" && d.w > 0.0 && d.h > 0.0)
            .collect()
    }

    /// Rasterize last paint into a softbuffer pixel buffer (CPU path).
    pub fn rasterize_softbuffer(&self, pixels: &mut [u32], ww: u32, wh: u32) -> bool {
        let Some(list) = self.last_paint.as_ref() else {
            return false;
        };
        rasterize_product_paint(list, pixels, ww, wh);
        true
    }

    /// Convenience: present + rasterize in one call (softbuffer hosts).
    pub fn present_session_softbuffer(
        &mut self,
        session: &VnSession,
        pixels: &mut [u32],
        ww: u32,
        wh: u32,
    ) -> &ProductPaintList {
        let _ = self.present_session(session);
        let _ = self.rasterize_softbuffer(pixels, ww, wh);
        self.last_paint.as_ref().expect("present_session set paint")
    }

    /// Extract product text paint commands as GPU layout inputs
    /// `(text, x, y, size, color, z)`.
    pub fn text_layout_items(&self) -> Vec<(String, f32, f32, f32, [f32; 4], f32)> {
        let Some(list) = self.last_paint.as_ref() else {
            return Vec::new();
        };
        list.commands
            .iter()
            .filter_map(|c| match c {
                crate::product_paint::ProductPaintCmd::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                    z,
                    ..
                } if !text.is_empty() && *size > 0.0 => {
                    Some((text.clone(), *x, *y, *size, *color, *z))
                }
                _ => None,
            })
            .collect()
    }

    /// Layout last paint text with velvet-text GPU rasterizer (product path).
    ///
    /// Returns `None` if no font is available or there is no text.
    pub fn layout_gpu_text(
        &self,
        raster: &mut velvet_text::GpuTextRasterizer,
    ) -> Option<Vec<velvet_text::GpuTextRun>> {
        let items = self.text_layout_items();
        if items.is_empty() {
            return None;
        }
        Some(velvet_text::layout_product_text_items(raster, &items))
    }

    /// Summary line for logs / headless ASSERT_OK.
    pub fn status_line(&self) -> String {
        let paint_n = self.last_paint.as_ref().map(|p| p.len()).unwrap_or(0);
        let quad_n = self.gpu_quad_descriptors().len();
        let text_n = self.gpu_text_descriptors().len();
        let fb = if self.gpu_fallback {
            format!(
                " fallback={}",
                self.fallback_reason.as_deref().unwrap_or("gpu_unavailable")
            )
        } else {
            String::new()
        };
        format!(
            "presenter phase={:?} preferred={} active={} paint={paint_n} quads={quad_n} text={text_n}{fb}",
            self.phase,
            self.preferred.as_str(),
            self.active.as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::runtime::{StoryPlayer, StoryWait};

    fn sample_session() -> VnSession {
        let src = r#"
character hero { name: "Hero" }
scene main {
    background "bg/station.png"
    show hero at left
    hero "Presenter path."
}
"#;
        let program = load_program_from_source(src, Some("presenter.vel"), "P").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        session
    }

    #[test]
    fn hybrid_title_is_softbuffer_play_is_wgpu() {
        let mut p = ProductPresenter::hybrid();
        assert_eq!(p.phase(), PresenterPhase::Title);
        assert_eq!(p.active(), PresenterBackend::Softbuffer);

        p.set_phase_play();
        assert_eq!(p.active(), PresenterBackend::Wgpu);

        p.set_phase_title();
        assert_eq!(p.active(), PresenterBackend::Softbuffer);
    }

    #[test]
    fn gpu_fallback_forces_softbuffer_on_play() {
        let mut p = ProductPresenter::hybrid();
        p.set_phase_play();
        assert_eq!(p.active(), PresenterBackend::Wgpu);
        p.set_gpu_available(false, Some("no adapter"));
        assert_eq!(p.active(), PresenterBackend::Softbuffer);
        assert!(p.gpu_fallback());
        assert_eq!(p.fallback_reason(), Some("no adapter"));
    }

    #[test]
    fn present_session_builds_paint_and_descriptors() {
        let session = sample_session();
        let mut p = ProductPresenter::hybrid();
        p.set_phase_play();
        let list = p.present_session(&session);
        assert!(!list.is_empty());
        assert!(list.has_say_geometry() || !p.last_descriptors().is_empty());
        assert!(
            !p.gpu_quad_descriptors().is_empty(),
            "expected GPU quads, status={}",
            p.status_line()
        );
        // Softbuffer raster must still work for hybrid fallback.
        let mut pixels = vec![0u32; 1280 * 720];
        assert!(p.rasterize_softbuffer(&mut pixels, 1280, 720));
        assert!(pixels.iter().any(|&c| c != 0));
    }

    #[test]
    fn status_line_mentions_backend() {
        let mut p = ProductPresenter::hybrid();
        p.set_phase_play();
        let s = p.status_line();
        assert!(s.contains("wgpu") || s.contains("softbuffer"), "{s}");
    }

    #[test]
    fn product_text_uses_velvet_text_gpu_path() {
        let session = sample_session();
        let mut p = ProductPresenter::hybrid();
        p.set_phase_play();
        let _ = p.present_session(&session);
        let items = p.text_layout_items();
        assert!(
            !items.is_empty(),
            "expected product text items, descs={:?}",
            p.last_descriptors()
        );
        let Ok(mut raster) = velvet_text::GpuTextRasterizer::from_system_ui() else {
            eprintln!("phase2: skip layout — no system font");
            return;
        };
        let runs = p.layout_gpu_text(&mut raster).expect("text runs");
        let flat = velvet_text::flatten_glyph_quads(&runs);
        assert!(
            !flat.is_empty(),
            "velvet-text GPU path must produce glyph quads"
        );
        // Not softbuffer bitmap: real atlas coverage
        let atlas = raster.atlas();
        assert!(atlas.rgba.iter().skip(3).step_by(4).any(|&a| a > 16));
    }
}
