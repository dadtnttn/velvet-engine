//! Screen layers for Studio — stacked “pantallas” with per-layer pixel resolution.
//!
//! Lower layers sit under higher `z`. Only the **active** layer is editable;
//! layers below are blocked (visual ghost). Switching layers with a different
//! resolution drives a short resize animation on the design surface.

use std::path::PathBuf;

/// One screen layer (a full UI surface / pantallas stack entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenLayer {
    /// Stable id (`main_menu`, `hud`, `mobile_overlay`).
    pub id: String,
    /// Human label.
    pub name: String,
    /// Draw / stack order (higher = on top).
    pub z: i32,
    /// When true, layer cannot become editable until unlocked.
    pub locked: bool,
    /// Drawn in the stack (ghost when inactive).
    pub visible: bool,
    /// Design width in logical pixels.
    pub width_px: u32,
    /// Design height in logical pixels.
    pub height_px: u32,
    /// Optional separate `.vel` document for this screen.
    pub document_path: Option<PathBuf>,
}

impl ScreenLayer {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        z: i32,
        width_px: u32,
        height_px: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            z,
            locked: false,
            visible: true,
            width_px: width_px.max(64),
            height_px: height_px.max(64),
            document_path: None,
        }
    }

    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    pub fn with_document(mut self, path: impl Into<PathBuf>) -> Self {
        self.document_path = Some(path.into());
        self
    }
}

/// Common design resolutions (desktop / mobile).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResPreset {
    DesktopHd,
    DesktopFhd,
    MobilePortrait,
    MobileLandscape,
    Square,
}

impl ResPreset {
    pub fn size(self) -> (u32, u32) {
        match self {
            Self::DesktopHd => (1280, 720),
            Self::DesktopFhd => (1920, 1080),
            Self::MobilePortrait => (390, 844),
            Self::MobileLandscape => (844, 390),
            Self::Square => (720, 720),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DesktopHd => "1280x720",
            Self::DesktopFhd => "1920x1080",
            Self::MobilePortrait => "390x844 phone",
            Self::MobileLandscape => "844x390 land",
            Self::Square => "720x720",
        }
    }
}

/// Percent (0..=100) ↔ pixel helpers for a layer resolution.
pub fn pct_to_px(x_pct: f32, y_pct: f32, width_px: u32, height_px: u32) -> (i32, i32) {
    let x = (x_pct.clamp(0.0, 100.0) / 100.0 * width_px as f32).round() as i32;
    let y = (y_pct.clamp(0.0, 100.0) / 100.0 * height_px as f32).round() as i32;
    (x, y)
}

pub fn px_to_pct(x_px: f32, y_px: f32, width_px: u32, height_px: u32) -> (f32, f32) {
    let w = width_px.max(1) as f32;
    let h = height_px.max(1) as f32;
    (
        (x_px / w * 100.0).clamp(0.0, 100.0),
        (y_px / h * 100.0).clamp(0.0, 100.0),
    )
}

/// Animated transition between two layer resolutions.
#[derive(Debug, Clone, Copy)]
pub struct LayerResizeAnim {
    pub from_w: f32,
    pub from_h: f32,
    pub to_w: f32,
    pub to_h: f32,
    /// 0.0 .. 1.0
    pub t: f32,
    /// Duration in seconds.
    pub duration: f32,
}

impl LayerResizeAnim {
    pub fn start(from_w: u32, from_h: u32, to_w: u32, to_h: u32) -> Self {
        Self {
            from_w: from_w as f32,
            from_h: from_h as f32,
            to_w: to_w as f32,
            to_h: to_h as f32,
            t: 0.0,
            duration: 0.28,
        }
    }

    /// Ease-out cubic progress.
    pub fn ease(self) -> f32 {
        let t = self.t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    pub fn current_size(self) -> (f32, f32) {
        let e = self.ease();
        (
            self.from_w + (self.to_w - self.from_w) * e,
            self.from_h + (self.to_h - self.from_h) * e,
        )
    }

    pub fn finished(self) -> bool {
        self.t >= 1.0
    }

    /// Advance by `dt` seconds; returns true while still running.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.duration <= 0.0 {
            self.t = 1.0;
            return false;
        }
        self.t = (self.t + dt / self.duration).min(1.0);
        !self.finished()
    }
}

/// Stack of screen layers for a Studio project.
#[derive(Debug, Clone)]
pub struct LayerStack {
    pub layers: Vec<ScreenLayer>,
    pub active_id: String,
    pub resize_anim: Option<LayerResizeAnim>,
}

impl Default for LayerStack {
    fn default() -> Self {
        Self::desktop_menu_stack()
    }
}

impl LayerStack {
    /// Default VN stack: locked main menu at bottom + free overlay on top.
    pub fn desktop_menu_stack() -> Self {
        let layers = vec![
            ScreenLayer::new("main_menu", "Main Menu", 0, 1280, 720).with_locked(true),
            ScreenLayer::new("overlay", "Overlay / HUD", 10, 1280, 720),
        ];
        Self {
            active_id: "main_menu".into(),
            layers,
            resize_anim: None,
        }
    }

    /// Menu + mobile portrait layer for phone-style games.
    pub fn with_mobile_preset() -> Self {
        let layers = vec![
            ScreenLayer::new("main_menu", "Main Menu", 0, 1280, 720).with_locked(true),
            ScreenLayer::new("mobile", "Mobile UI", 20, 390, 844),
        ];
        Self {
            active_id: "main_menu".into(),
            layers,
            resize_anim: None,
        }
    }

    pub fn sorted_ids(&self) -> Vec<String> {
        let mut v: Vec<_> = self.layers.iter().collect();
        v.sort_by_key(|l| l.z);
        v.into_iter().map(|l| l.id.clone()).collect()
    }

    pub fn active(&self) -> Option<&ScreenLayer> {
        self.layers.iter().find(|l| l.id == self.active_id)
    }

    pub fn active_mut(&mut self) -> Option<&mut ScreenLayer> {
        let id = self.active_id.clone();
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&ScreenLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ScreenLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Logical resolution currently shown (animated mid-transition if any).
    pub fn display_resolution(&self) -> (f32, f32) {
        if let Some(anim) = self.resize_anim {
            return anim.current_size();
        }
        self.active()
            .map(|l| (l.width_px as f32, l.height_px as f32))
            .unwrap_or((1280.0, 720.0))
    }

    pub fn active_resolution(&self) -> (u32, u32) {
        self.active()
            .map(|l| (l.width_px, l.height_px))
            .unwrap_or((1280, 720))
    }

    /// Switch active layer; starts resize anim if resolution changes.
    pub fn set_active(&mut self, id: &str) -> Result<(), String> {
        let layer = self
            .get(id)
            .ok_or_else(|| format!("unknown layer {id}"))?
            .clone();
        if layer.locked && layer.id != self.active_id {
            // Allow selecting locked layer only if we unlock first — still allow
            // focus for viewing; editing is gated elsewhere.
        }
        let (fw, fh) = self.display_resolution();
        let (tw, th) = (layer.width_px, layer.height_px);
        if (fw - tw as f32).abs() > 0.5 || (fh - th as f32).abs() > 0.5 {
            self.resize_anim = Some(LayerResizeAnim::start(
                fw.round() as u32,
                fh.round() as u32,
                tw,
                th,
            ));
        } else {
            self.resize_anim = None;
        }
        self.active_id = layer.id;
        Ok(())
    }

    /// Cycle next layer by z-order.
    pub fn cycle_next(&mut self) -> Result<String, String> {
        let ids = self.sorted_ids();
        if ids.is_empty() {
            return Err("no layers".into());
        }
        let idx = ids.iter().position(|i| i == &self.active_id).unwrap_or(0);
        let next = &ids[(idx + 1) % ids.len()];
        self.set_active(next)?;
        Ok(next.clone())
    }

    pub fn cycle_prev(&mut self) -> Result<String, String> {
        let ids = self.sorted_ids();
        if ids.is_empty() {
            return Err("no layers".into());
        }
        let idx = ids.iter().position(|i| i == &self.active_id).unwrap_or(0);
        let prev = &ids[(idx + ids.len() - 1) % ids.len()];
        self.set_active(prev)?;
        Ok(prev.clone())
    }

    /// Toggle lock on active layer.
    pub fn toggle_lock_active(&mut self) -> bool {
        if let Some(l) = self.active_mut() {
            l.locked = !l.locked;
            return l.locked;
        }
        false
    }

    /// Whether the active layer accepts edits.
    pub fn active_editable(&self) -> bool {
        self.active().map(|l| !l.locked).unwrap_or(false)
    }

    /// Set resolution of active layer (starts anim).
    pub fn set_active_resolution(&mut self, w: u32, h: u32) -> Result<(), String> {
        let (fw, fh) = self.display_resolution();
        let w = w.max(64);
        let h = h.max(64);
        {
            let layer = self
                .active_mut()
                .ok_or_else(|| "no active layer".to_string())?;
            layer.width_px = w;
            layer.height_px = h;
        }
        self.resize_anim = Some(LayerResizeAnim::start(
            fw.round() as u32,
            fh.round() as u32,
            w,
            h,
        ));
        Ok(())
    }

    pub fn apply_preset(&mut self, preset: ResPreset) -> Result<(), String> {
        let (w, h) = preset.size();
        self.set_active_resolution(w, h)
    }

    /// Add a layer above the top.
    pub fn add_layer(&mut self, id: &str, name: &str, w: u32, h: u32) -> Result<(), String> {
        if self.layers.iter().any(|l| l.id == id) {
            return Err(format!("layer id exists: {id}"));
        }
        let z = self.layers.iter().map(|l| l.z).max().unwrap_or(0) + 10;
        self.layers
            .push(ScreenLayer::new(id, name, z, w, h));
        self.set_active(id)?;
        Ok(())
    }

    /// Layers below active (lower z), visible — for ghost paint.
    pub fn layers_below_active(&self) -> Vec<&ScreenLayer> {
        let z = self.active().map(|l| l.z).unwrap_or(0);
        let mut v: Vec<_> = self
            .layers
            .iter()
            .filter(|l| l.visible && l.z < z)
            .collect();
        v.sort_by_key(|l| l.z);
        v
    }

    pub fn tick_anim(&mut self, dt: f32) -> bool {
        if let Some(ref mut anim) = self.resize_anim {
            let running = anim.tick(dt);
            if !running {
                self.resize_anim = None;
            }
            return running;
        }
        false
    }
}

/// Letterbox a design surface of aspect `res_w:res_h` inside `canvas_*`.
#[derive(Debug, Clone, Copy)]
pub struct DesignSurface {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub res_w: f32,
    pub res_h: f32,
}

impl DesignSurface {
    pub fn fit(canvas_x: i32, canvas_y: i32, canvas_w: i32, canvas_h: i32, res_w: f32, res_h: f32) -> Self {
        let res_w = res_w.max(1.0);
        let res_h = res_h.max(1.0);
        let canvas_w = canvas_w.max(1) as f32;
        let canvas_h = canvas_h.max(1) as f32;
        let scale = (canvas_w / res_w).min(canvas_h / res_h);
        let w = (res_w * scale).round().max(8.0) as i32;
        let h = (res_h * scale).round().max(8.0) as i32;
        let x = canvas_x + (canvas_w as i32 - w) / 2;
        let y = canvas_y + (canvas_h as i32 - h) / 2;
        Self {
            x,
            y,
            w,
            h,
            res_w,
            res_h,
        }
    }

    pub fn contains(&self, sx: f64, sy: f64) -> bool {
        let x = sx as i32;
        let y = sy as i32;
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    /// Screen → design percent.
    pub fn screen_to_pct(&self, sx: f64, sy: f64) -> (f32, f32) {
        let px = ((sx as f32 - self.x as f32) / self.w as f32 * 100.0).clamp(0.0, 100.0);
        let py = ((sy as f32 - self.y as f32) / self.h as f32 * 100.0).clamp(0.0, 100.0);
        (px, py)
    }

    pub fn screen_delta_to_pct(&self, dx: f64, dy: f64) -> (f32, f32) {
        (
            dx as f32 / self.w.max(1) as f32 * 100.0,
            dy as f32 / self.h.max(1) as f32 * 100.0,
        )
    }

    /// Design percent → screen pixel center.
    pub fn pct_to_screen(&self, x_pct: f32, y_pct: f32) -> (i32, i32) {
        let x = self.x + ((x_pct / 100.0) * self.w as f32) as i32;
        let y = self.y + ((y_pct / 100.0) * self.h as f32) as i32;
        (x, y)
    }

    /// Design percent → logical layer pixels.
    pub fn pct_to_layer_px(&self, x_pct: f32, y_pct: f32) -> (i32, i32) {
        pct_to_px(x_pct, y_pct, self.res_w.round() as u32, self.res_h.round() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_switch_starts_anim_when_res_changes() {
        let mut s = LayerStack::with_mobile_preset();
        assert_eq!(s.active_id, "main_menu");
        s.set_active("mobile").unwrap();
        assert!(s.resize_anim.is_some());
        assert_eq!(s.active_id, "mobile");
        // advance anim to end
        for _ in 0..30 {
            s.tick_anim(0.05);
        }
        assert!(s.resize_anim.is_none());
        let (w, h) = s.active_resolution();
        assert_eq!((w, h), (390, 844));
    }

    #[test]
    fn pct_px_roundtrip() {
        let (x, y) = pct_to_px(50.0, 25.0, 200, 400);
        assert_eq!((x, y), (100, 100));
        let (px, py) = px_to_pct(100.0, 100.0, 200, 400);
        assert!((px - 50.0).abs() < 0.01);
        assert!((py - 25.0).abs() < 0.01);
    }

    #[test]
    fn locked_main_menu_default() {
        let s = LayerStack::desktop_menu_stack();
        assert!(s.get("main_menu").unwrap().locked);
        assert!(!s.active_editable()); // main_menu locked
    }

    #[test]
    fn design_surface_letterbox() {
        let ds = DesignSurface::fit(0, 0, 1000, 500, 1280.0, 720.0);
        assert!(ds.w <= 1000 && ds.h <= 500);
        assert!((ds.w as f32 / ds.h as f32 - 1280.0 / 720.0).abs() < 0.05);
    }
}
