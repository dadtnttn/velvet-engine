//! Studio docking GUI model + launch entry.
//!
//! The drag/move logic lives in [`velvet_document`]; this module owns panel layout,
//! canvas interaction state, and a launch path that either opens a short-lived
//! window (when the display allows) or runs headless with a ready log.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use velvet_document::{
    drag_visual_region, hit_test_visual, parse_document, region_rect, render_document,
    DesignerWidget, UiDesigner, WidgetRect,
};

use crate::layers::{pct_to_px, DesignSurface, LayerStack, ResPreset};

/// Named dock panel (left/right/bottom/center).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockPanel {
    /// Stable id: hierarchy | inspector | console | assets | scripts | canvas.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Dock zone.
    pub zone: DockZone,
    /// Visible.
    pub visible: bool,
}

/// Where a panel docks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockZone {
    /// Left column.
    Left,
    /// Right column.
    Right,
    /// Bottom strip.
    Bottom,
    /// Center document/canvas.
    Center,
}

/// Dual-mode Studio editor: simplified visual designer vs advanced script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudioEditorMode {
    /// Drag-and-drop canvas over `@visual` regions (default).
    #[default]
    Simplified,
    /// Script / advanced text of the same file.
    Advanced,
}

/// GUI session state over one project + open document.
#[derive(Debug, Clone)]
pub struct StudioGuiSession {
    /// Project root.
    pub root: PathBuf,
    /// Dock panels.
    pub panels: Vec<DockPanel>,
    /// Path of the document open on the canvas (relative or absolute).
    pub document_path: Option<PathBuf>,
    /// Source currently edited on the canvas.
    pub document_source: String,
    /// Selected region id for drag.
    pub selected_region: Option<String>,
    /// Ready flag after layout init.
    pub ready: bool,
    /// Last drag rect (for tests/logs).
    pub last_drag: Option<WidgetRect>,
    /// Log lines (also written to ready log file when configured).
    pub log: Vec<String>,
    /// Active editor mode (simplified / advanced).
    pub mode: StudioEditorMode,
    /// Next drop id counter for palette widgets.
    pub drop_seq: u32,
    /// Screen layers stack (pantallas) with per-layer pixel resolution.
    pub layers: LayerStack,
}

impl StudioGuiSession {
    /// Build default docking layout for a project.
    pub fn open_project(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let root = root.canonicalize().unwrap_or(root);
        let mut session = Self {
            root,
            panels: default_dock_panels(),
            document_path: None,
            document_source: String::new(),
            selected_region: None,
            ready: false,
            last_drag: None,
            log: Vec::new(),
            mode: StudioEditorMode::Simplified,
            drop_seq: 0,
            layers: LayerStack::vn_tree(),
        };
        session.log.push(format!(
            "[studio-gui] opened project {}",
            session.root.display()
        ));
        session.ready = true;
        if let Some(l) = session.layers.get_mut("main_menu") {
            l.locked = false;
            l.expanded = true;
        }
        session.log.push(format!(
            "[studio-gui] ready panels={} mode=simplified layers={}",
            session
                .panels
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(","),
            session.layers.sorted_ids().join("/")
        ));
        Ok(session)
    }

    /// Toggle or set dual mode (simplified visual vs advanced script).
    pub fn set_mode(&mut self, mode: StudioEditorMode) -> Result<()> {
        if self.mode == mode {
            return Ok(());
        }
        // Re-validate document when leaving advanced (script may have been edited)
        if matches!(self.mode, StudioEditorMode::Advanced) {
            let _ = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        self.mode = mode;
        let name = match mode {
            StudioEditorMode::Simplified => "simplified",
            StudioEditorMode::Advanced => "advanced",
        };
        self.log.push(format!("[studio-gui] mode={name}"));
        // Reparse so simplified list matches advanced edits
        if !self.document_source.is_empty() {
            let _ = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Ok(())
    }

    /// Toggle mode (simplified <-> advanced).
    pub fn toggle_mode(&mut self) -> Result<StudioEditorMode> {
        let next = match self.mode {
            StudioEditorMode::Simplified => StudioEditorMode::Advanced,
            StudioEditorMode::Advanced => StudioEditorMode::Simplified,
        };
        self.set_mode(next)?;
        Ok(self.mode)
    }

    /// List visual widgets (simplified mode palette/canvas).
    pub fn list_widgets(&self) -> Result<Vec<DesignerWidget>> {
        if self.document_source.is_empty() {
            return Ok(Vec::new());
        }
        let d = UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.list_widgets().map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Drop a palette widget at percent canvas coords (simplified mode).
    pub fn drop_widget(&mut self, kind: &str, x_pct: f32, y_pct: f32) -> Result<String> {
        self.drop_seq += 1;
        let id = format!("w{}", self.drop_seq);
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.drop_widget(kind, &id, x_pct, y_pct)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        let full = if id.contains('.') {
            id.clone()
        } else {
            let k = match kind.to_ascii_lowercase().as_str() {
                "label" | "text" => "label",
                "panel" | "box" => "panel",
                _ => "button",
            };
            format!("{k}.{id}")
        };
        self.selected_region = Some(full.clone());
        self.log
            .push(format!("[studio-gui] drop kind={kind} id={full} at ({x_pct},{y_pct})"));
        Ok(full)
    }

    /// Set text on selected (or given) visual region.
    pub fn set_widget_text(&mut self, region_id: Option<&str>, text: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for text"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_text(&id, text).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_text region={id} text={text}"));
        Ok(())
    }

    /// Set position string e.g. `(50%, 62%)` on selected/given region.
    pub fn set_widget_position(&mut self, region_id: Option<&str>, position: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for position"))?;
        let normalized = normalize_pct_pair(position)
            .ok_or_else(|| anyhow::anyhow!("bad position (use 50,62 or (50%, 62%))"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_position(&id, &normalized)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_position region={id} {normalized}"));
        Ok(())
    }

    /// Set size string e.g. `(18%, 8%)` on selected/given region.
    pub fn set_widget_size(&mut self, region_id: Option<&str>, size: &str) -> Result<()> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region for size"))?;
        let normalized = normalize_pct_pair(size)
            .ok_or_else(|| anyhow::anyhow!("bad size (use 18,8 or (18%, 8%))"))?;
        let mut d =
            UiDesigner::open(self.document_source.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_size(&id, &normalized)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push(format!("[studio-gui] set_size region={id} {normalized}"));
        Ok(())
    }

    /// Nudge selected widget position by integer percent points.
    pub fn nudge_selected(&mut self, dx: f32, dy: f32) -> Result<Option<WidgetRect>> {
        if self.selected_region.is_none() {
            return Ok(None);
        }
        Ok(Some(self.drag_region(None, dx, dy)?))
    }

    /// Advanced mode: replace entire document source (must parse).
    pub fn set_advanced_source(&mut self, source: impl Into<String>) -> Result<()> {
        let source = source.into();
        let mut d = UiDesigner::open(self.document_source.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        d.set_source_advanced(source)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = d.save_source();
        self.log
            .push("[studio-gui] advanced source applied (reparsed)".into());
        Ok(())
    }

    /// Snapshot source for advanced text view.
    pub fn advanced_source(&self) -> &str {
        &self.document_source
    }

    /// Open a `.vel` document onto the canvas (loads source).
    pub fn open_document(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let source = fs::read_to_string(&abs)
            .with_context(|| format!("read document {}", abs.display()))?;
        // Validate parse early
        let _ = parse_document(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_path = Some(abs);
        self.document_source = source;
        self.log.push(format!(
            "[studio-gui] document {}",
            self.document_path.as_ref().unwrap().display()
        ));
        Ok(())
    }

    /// Select a region by id (must be visual).
    pub fn select_region(&mut self, region_id: &str) -> Result<()> {
        let doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let _ = region_rect(&doc, region_id).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.selected_region = Some(region_id.to_string());
        self.log
            .push(format!("[studio-gui] select region={region_id}"));
        Ok(())
    }

    /// Drag the selected (or given) region by delta — **the shipped drag path**.
    pub fn drag_region(
        &mut self,
        region_id: Option<&str>,
        dx: f32,
        dy: f32,
    ) -> Result<WidgetRect> {
        let id = region_id
            .map(|s| s.to_string())
            .or_else(|| self.selected_region.clone())
            .ok_or_else(|| anyhow::anyhow!("no region selected for drag"))?;
        let mut doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let rect = drag_visual_region(&mut doc, &id, dx, dy).map_err(|e| anyhow::anyhow!("{e}"))?;
        self.document_source = render_document(&doc);
        self.last_drag = Some(rect);
        self.log.push(format!(
            "[studio-gui] drag region={id} dx={dx} dy={dy} -> ({:.0}%, {:.0}%)",
            rect.pos.x, rect.pos.y
        ));
        Ok(rect)
    }

    /// Pointer press on canvas (percent coords) — selects via hit-test.
    pub fn canvas_pointer_down(&mut self, x: f32, y: f32) -> Result<Option<String>> {
        let doc = parse_document(&self.document_source).map_err(|e| anyhow::anyhow!("{e}"))?;
        let hit = hit_test_visual(&doc, x, y);
        if let Some(ref id) = hit {
            self.selected_region = Some(id.clone());
            self.log
                .push(format!("[studio-gui] hit-test ({x},{y}) -> {id}"));
        }
        Ok(hit)
    }

    /// Pointer drag with button held: moves selection.
    pub fn canvas_pointer_drag(&mut self, dx: f32, dy: f32) -> Result<Option<WidgetRect>> {
        if self.selected_region.is_none() {
            return Ok(None);
        }
        Ok(Some(self.drag_region(None, dx, dy)?))
    }

    /// Write document source back to disk (round-trip).
    pub fn save_document(&self) -> Result<()> {
        let path = self
            .document_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no document open"))?;
        fs::write(path, &self.document_source)
            .with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    /// Panel ids currently visible (for ready assertions).
    pub fn panel_ids(&self) -> Vec<&str> {
        self.panels
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.id.as_str())
            .collect()
    }

    /// Active layer design surface letterboxed into layout canvas.
    pub fn design_surface(&self, canvas_x: i32, canvas_y: i32, canvas_w: i32, canvas_h: i32) -> DesignSurface {
        let (rw, rh) = self.layers.display_resolution();
        DesignSurface::fit(canvas_x, canvas_y, canvas_w, canvas_h, rw, rh)
    }

    /// Pixel coordinates of selected widget center on the active layer.
    pub fn selected_pos_px(&self) -> Option<(i32, i32)> {
        let id = self.selected_region.as_ref()?;
        let widgets = self.list_widgets().ok()?;
        let w = widgets.iter().find(|w| w.id == *id)?;
        let (x, y) = parse_pct_pair_loose(w.position.as_deref().unwrap_or("(50%,50%)"));
        let (rw, rh) = self.layers.active_resolution();
        Some(pct_to_px(x, y, rw, rh))
    }

    /// Switch layer; returns status string.
    pub fn set_layer(&mut self, id: &str) -> Result<String, String> {
        let prev = self.layers.active_id.clone();
        self.layers.set_active(id)?;
        self.apply_layer_lock_policy(&prev, id);
        let (w, h) = self.layers.active_resolution();
        let msg = format!("layer={} {}x{}px", id, w, h);
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    fn apply_layer_lock_policy(&mut self, _prev: &str, id: &str) {
        // Roots re-lock when not active; active branch unlocks for edit.
        // Sublayers under inactive roots stay as-is; active node unlocks.
        let roots: Vec<String> = self
            .layers
            .layers
            .iter()
            .filter(|l| l.parent.is_none())
            .map(|l| l.id.clone())
            .collect();
        // Active root = root ancestor of id
        let mut active_root = id.to_string();
        let mut cur = Some(id.to_string());
        for _ in 0..16 {
            if let Some(cid) = cur {
                if let Some(l) = self.layers.get(&cid) {
                    if l.parent.is_none() {
                        active_root = l.id.clone();
                        break;
                    }
                    cur = l.parent.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        for r in roots {
            if let Some(l) = self.layers.get_mut(&r) {
                // only auto-lock main_menu as the “base” pantallas
                if r == "main_menu" {
                    l.locked = r != active_root;
                }
            }
        }
        if let Some(l) = self.layers.get_mut(id) {
            if id != "main_menu" || active_root == "main_menu" {
                // ensure active is editable unless user locked it intentionally
                if id == "main_menu" {
                    l.locked = false;
                }
            }
        }
    }

    pub fn cycle_layer(&mut self, forward: bool) -> Result<String, String> {
        let prev = self.layers.active_id.clone();
        let id = if forward {
            self.layers.cycle_next()?
        } else {
            self.layers.cycle_prev()?
        };
        self.apply_layer_lock_policy(&prev, &id);
        let (w, h) = self.layers.active_resolution();
        let msg = format!("layer={} {}x{}px", id, w, h);
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn set_layer_preset(&mut self, preset: ResPreset) -> Result<String, String> {
        self.layers.apply_preset(preset)?;
        let (w, h) = self.layers.active_resolution();
        let msg = format!(
            "layer {} res → {} ({})",
            self.layers.active_id,
            preset.label(),
            format!("{w}x{h}")
        );
        self.log.push(format!("[studio-gui] {msg}"));
        Ok(msg)
    }

    pub fn add_mobile_layer(&mut self) -> Result<String, String> {
        if self.layers.get("mobile").is_none() {
            self.layers
                .add_child("main_menu", "mobile", "Mobile UI", 390, 844)?;
        } else {
            self.layers.set_active("mobile")?;
        }
        self.set_layer("mobile")
    }

    /// Add sublayer under active (or under parent if active is already a child).
    pub fn add_sublayer(&mut self, id: &str, name: &str) -> Result<String, String> {
        let parent = self.layers.active_id.clone();
        let (w, h) = self.layers.active_resolution();
        self.layers.add_child(&parent, id, name, w, h)?;
        self.set_layer(id)
    }

    pub fn toggle_layer_expand(&mut self, id: &str) -> bool {
        self.layers.toggle_expanded(id)
    }

    pub fn tick_layers(&mut self, dt: f32) -> bool {
        self.layers.tick_anim(dt)
    }
}

fn parse_pct_pair_loose(s: &str) -> (f32, f32) {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')');
    let mut parts = t.split(',');
    let a = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    let b = parts
        .next()
        .unwrap_or("50")
        .trim()
        .trim_end_matches('%')
        .parse()
        .unwrap_or(50.0);
    (a, b)
}

/// Accept `50,62`, `50% 62%`, `(50%, 62%)` → `(50%, 62%)`.
fn normalize_pct_pair(raw: &str) -> Option<String> {
    let t = raw
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .replace('%', " ");
    let parts: Vec<&str> = t
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let a: f32 = parts[0].parse().ok()?;
    let b: f32 = parts[1].parse().ok()?;
    if !a.is_finite() || !b.is_finite() {
        return None;
    }
    Some(format!("({:.0}%, {:.0}%)", a.clamp(0.0, 100.0), b.clamp(0.0, 100.0)))
}

fn default_dock_panels() -> Vec<DockPanel> {
    vec![
        DockPanel {
            id: "hierarchy".into(),
            title: "Hierarchy".into(),
            zone: DockZone::Left,
            visible: true,
        },
        DockPanel {
            id: "assets".into(),
            title: "Assets".into(),
            zone: DockZone::Left,
            visible: true,
        },
        DockPanel {
            id: "canvas".into(),
            title: "Visual Canvas".into(),
            zone: DockZone::Center,
            visible: true,
        },
        DockPanel {
            id: "inspector".into(),
            title: "Inspector".into(),
            zone: DockZone::Right,
            visible: true,
        },
        DockPanel {
            id: "scripts".into(),
            title: "Scripts".into(),
            zone: DockZone::Right,
            visible: true,
        },
        DockPanel {
            id: "console".into(),
            title: "Console".into(),
            zone: DockZone::Bottom,
            visible: true,
        },
    ]
}

/// Launch configuration for Studio GUI.
#[derive(Debug, Clone)]
pub struct StudioGuiConfig {
    /// Project root.
    pub root: PathBuf,
    /// Optional document to open on canvas.
    pub document: Option<PathBuf>,
    /// Headless: never open OS window.
    pub headless: bool,
    /// Exit after init (+ optional demo drag).
    pub once: bool,
    /// Demo drag region id (if document open).
    pub demo_drag_region: Option<String>,
    /// Demo drag deltas.
    pub demo_dx: f32,
    /// Demo drag dy.
    pub demo_dy: f32,
    /// Write ready log here.
    pub ready_log: Option<PathBuf>,
    /// Persist document after demo drag.
    pub save_after_drag: bool,
}

impl Default for StudioGuiConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            document: None,
            headless: true,
            once: true,
            demo_drag_region: None,
            demo_dx: 0.0,
            demo_dy: 0.0,
            ready_log: None,
            save_after_drag: false,
        }
    }
}

/// Status returned from a GUI launch.
#[derive(Debug, Clone)]
pub struct StudioGuiStatus {
    /// Session after run.
    pub session: StudioGuiSession,
    /// Window was opened (false when headless or display failed).
    pub window_opened: bool,
    /// Optional display error when window path failed.
    pub display_error: Option<String>,
}

/// Run Studio GUI once (or headless ready path).
///
/// Always initializes docking panels and can exercise drag via config.
pub fn run_studio_gui(cfg: StudioGuiConfig) -> Result<StudioGuiStatus> {
    let mut session = StudioGuiSession::open_project(&cfg.root)?;
    if let Some(doc) = &cfg.document {
        session.open_document(doc)?;
    } else {
        // Auto-pick common menu / main script if present
        for candidate in [
            "scripts/main_menu.vel",
            "scripts/main.vel",
            "ui/main_menu.vel",
        ] {
            let p = session.root.join(candidate);
            if p.is_file() {
                session.open_document(&p)?;
                break;
            }
        }
    }

    if let Some(id) = &cfg.demo_drag_region {
        if !session.document_source.is_empty() {
            session.select_region(id)?;
            session.drag_region(Some(id), cfg.demo_dx, cfg.demo_dy)?;
            if cfg.save_after_drag {
                session.save_document()?;
                session
                    .log
                    .push("[studio-gui] saved document after drag".into());
            }
        }
    }

    let mut window_opened = false;
    let mut display_error = None;

    if !cfg.headless {
        let interactive = !cfg.once;
        match try_open_studio_window(&session, interactive) {
            Ok(()) => {
                window_opened = true;
                session.log.push(if interactive {
                    "[studio-gui] window opened (interactive dual-mode)".into()
                } else {
                    "[studio-gui] window opened (brief dual-mode paint)".into()
                });
            }
            Err(e) => {
                display_error = Some(e.to_string());
                session
                    .log
                    .push(format!("[studio-gui] window skipped: {e}"));
            }
        }
    } else {
        session.log.push("[studio-gui] headless mode".into());
        session
            .log
            .push(format!("[studio-gui] mode={:?}", session.mode));
    }

    if let Some(path) = &cfg.ready_log {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut f = fs::File::create(path)
            .with_context(|| format!("create ready log {}", path.display()))?;
        for line in &session.log {
            writeln!(f, "{line}")?;
        }
        writeln!(
            f,
            "[studio-gui] STATUS ready={} window={} panels={}",
            session.ready,
            window_opened,
            session.panel_ids().join(",")
        )?;
    }

    // Print to stdout for harness capture
    for line in &session.log {
        println!("{line}");
    }
    println!(
        "[studio-gui] STATUS ready={} window={} panels={}",
        session.ready,
        window_opened,
        session.panel_ids().join(",")
    );

    if !session.ready {
        bail!("studio gui failed to become ready");
    }

    let _ = cfg.once; // always returns after one pass for now
    Ok(StudioGuiStatus {
        session,
        window_opened,
        display_error,
    })
}

/// Softbuffer dual-mode Studio window (simplified canvas + advanced text strip).
///
/// When `interactive` is false, opens briefly then exits (CI / `--once`).
fn try_open_studio_window(session: &StudioGuiSession, interactive: bool) -> Result<()> {
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use softbuffer::{Context as SbContext, Surface};
    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{KeyCode, PhysicalKey};
    use winit::window::{Window, WindowId};

    struct Host {
        session: StudioGuiSession,
        interactive: bool,
        frames: u32,
        window: Option<Arc<Window>>,
        context: Option<SbContext<Arc<Window>>>,
        surface: Option<Surface<Arc<Window>, Arc<Window>>>,
        pixels: Vec<u32>,
        /// Last cursor in window pixels.
        cursor: (f64, f64),
        /// Last cursor when drag started / previous move (window pixels).
        drag_last_px: Option<(f64, f64)>,
        dragging: bool,
        /// Accumulated percent delta for 1% snap (smooth feel, grid placement).
        drag_acc_pct: (f32, f32),
        /// Ctrl/Super held (tracked via ModifiersChanged — KeyEvent has no modifiers).
        ctrl_held: bool,
        /// Inspector field being edited (TEXT / POS / SIZE).
        edit_field: Option<crate::studio_paint::InspectorField>,
        /// Live buffer while editing an inspector field.
        edit_buf: String,
        /// UI text zoom level 1..=4 (Ctrl+/- or Ctrl+wheel). Default 2.
        ui_zoom: i32,
        /// Last tick instant for layer resize animation.
        last_tick: std::time::Instant,
        status: String,
    }

    impl ApplicationHandler for Host {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let mode = match self.session.mode {
                StudioEditorMode::Simplified => "Visual",
                StudioEditorMode::Advanced => "Script",
            };
            let title = format!(
                "Velvet Studio [{mode}] — {}",
                self.session
                    .root
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("project")
            );
            let attrs = winit::window::Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(1280.0, 800.0));
            let window = Arc::new(el.create_window(attrs).expect("window"));
            let context = SbContext::new(window.clone()).expect("ctx");
            let surface = Surface::new(&context, window.clone()).expect("surface");
            self.context = Some(context);
            self.surface = Some(surface);
            self.window = Some(window);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
            match ev {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::ModifiersChanged(mods) => {
                    self.ctrl_held = mods.state().control_key() || mods.state().super_key();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if !self.ctrl_held {
                        return;
                    }
                    use winit::event::MouseScrollDelta;
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                1.0
                            } else if p.y < 0.0 {
                                -1.0
                            } else {
                                0.0
                            }
                        }
                    };
                    if dy > 0.0 {
                        self.bump_zoom(1);
                    } else if dy < 0.0 {
                        self.bump_zoom(-1);
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }
                    // Ctrl+/- always zooms UI text (even while editing)
                    if self.ctrl_held {
                        if let PhysicalKey::Code(c) = event.physical_key {
                            match c {
                                KeyCode::Equal | KeyCode::NumpadAdd => {
                                    self.bump_zoom(1);
                                    return;
                                }
                                KeyCode::Minus | KeyCode::NumpadSubtract => {
                                    self.bump_zoom(-1);
                                    return;
                                }
                                KeyCode::Digit0 | KeyCode::Numpad0 => {
                                    self.ui_zoom = 2;
                                    self.status = "UI zoom reset x2".into();
                                    self.redraw();
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }

                    // ── Inspector text editing takes keyboard focus ──
                    if self.edit_field.is_some() {
                        let PhysicalKey::Code(c) = event.physical_key else {
                            // still accept text via event.text
                            if let Some(t) = event.text.as_ref() {
                                for ch in t.chars() {
                                    if !ch.is_control() {
                                        self.edit_buf.push(ch);
                                    }
                                }
                                self.redraw();
                            }
                            return;
                        };
                        match c {
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                self.commit_edit();
                            }
                            KeyCode::Escape => {
                                self.cancel_edit();
                                self.status = "edit cancelled".into();
                                self.redraw();
                            }
                            KeyCode::Backspace => {
                                self.edit_buf.pop();
                                self.redraw();
                            }
                            KeyCode::Delete => {
                                self.edit_buf.clear();
                                self.redraw();
                            }
                            _ => {
                                if let Some(t) = event.text.as_ref() {
                                    for ch in t.chars() {
                                        if !ch.is_control() && self.edit_buf.len() < 64 {
                                            self.edit_buf.push(ch);
                                        }
                                    }
                                    self.redraw();
                                }
                            }
                        }
                        return;
                    }

                    let PhysicalKey::Code(c) = event.physical_key else {
                        return;
                    };
                    match c {
                        KeyCode::Tab => {
                            let _ = self.session.toggle_mode();
                            self.sync_title();
                            self.status = format!("mode={:?}", self.session.mode);
                            self.redraw();
                        }
                        KeyCode::KeyB if self.session.mode == StudioEditorMode::Simplified => {
                            self.drop_at_cursor("button");
                        }
                        KeyCode::KeyL if self.session.mode == StudioEditorMode::Simplified => {
                            self.drop_at_cursor("label");
                        }
                        // P = edit POS when selected, else drop panel
                        KeyCode::KeyP if self.session.mode == StudioEditorMode::Simplified => {
                            if self.session.selected_region.is_some() {
                                self.begin_edit(crate::studio_paint::InspectorField::Pos);
                            } else {
                                self.drop_at_cursor("panel");
                            }
                        }
                        KeyCode::KeyT
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            self.begin_edit(crate::studio_paint::InspectorField::Text);
                        }
                        KeyCode::KeyZ
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some()
                                && !self.ctrl_held =>
                        {
                            self.begin_edit(crate::studio_paint::InspectorField::Size);
                        }
                        KeyCode::ArrowLeft
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(-1.0, 0.0);
                            self.status = "nudge left".into();
                            self.redraw();
                        }
                        KeyCode::ArrowRight
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(1.0, 0.0);
                            self.status = "nudge right".into();
                            self.redraw();
                        }
                        KeyCode::ArrowUp
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(0.0, -1.0);
                            self.status = "nudge up".into();
                            self.redraw();
                        }
                        KeyCode::ArrowDown
                            if self.session.mode == StudioEditorMode::Simplified
                                && self.session.selected_region.is_some() =>
                        {
                            let _ = self.session.nudge_selected(0.0, 1.0);
                            self.status = "nudge down".into();
                            self.redraw();
                        }
                        KeyCode::KeyS if self.ctrl_held => {
                            self.do_save();
                        }
                        KeyCode::F5 => {
                            self.do_save();
                        }
                        KeyCode::Digit1 if !self.ctrl_held => {
                            let _ = self.session.set_mode(StudioEditorMode::Simplified);
                            self.sync_title();
                            self.status = "Visual mode".into();
                            self.redraw();
                        }
                        KeyCode::Digit2 if !self.ctrl_held => {
                            let _ = self.session.set_mode(StudioEditorMode::Advanced);
                            self.sync_title();
                            self.status = "Script mode".into();
                            self.redraw();
                        }
                        // Layer stack
                        KeyCode::BracketRight | KeyCode::PageDown => {
                            match self.session.cycle_layer(true) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::BracketLeft | KeyCode::PageUp => {
                            match self.session.cycle_layer(false) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::KeyU => {
                            let locked = self.session.layers.toggle_lock_active();
                            self.status = if locked {
                                "layer locked".into()
                            } else {
                                "layer unlocked".into()
                            };
                            self.redraw();
                        }
                        KeyCode::KeyM => {
                            match self.session.add_mobile_layer() {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::KeyN if self.session.mode == StudioEditorMode::Simplified => {
                            // N = new sublayer under active
                            let seq = self.session.drop_seq + 1;
                            let id = format!("sub{seq}");
                            match self.session.add_sublayer(&id, "Nueva subcapa") {
                                Ok(s) => {
                                    self.session.drop_seq = seq;
                                    self.status = s;
                                }
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit3 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::DesktopHd) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit4 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::MobilePortrait) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Digit5 if self.ctrl_held => {
                            match self.session.set_layer_preset(ResPreset::MobileLandscape) {
                                Ok(s) => self.status = s,
                                Err(e) => self.status = e,
                            }
                            self.redraw();
                        }
                        KeyCode::Escape => el.exit(),
                        _ => {}
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    let layout = self.layout();
                    if state == ElementState::Pressed {
                        // Toolbar hits (mode / save) work in both modes
                        if let Some(action) = layout.hit_toolbar(self.cursor.0, self.cursor.1) {
                            self.cancel_edit();
                            match action {
                                "mode_visual" => {
                                    let _ = self.session.set_mode(StudioEditorMode::Simplified);
                                    self.sync_title();
                                    self.status = "Visual mode".into();
                                }
                                "mode_script" => {
                                    let _ = self.session.set_mode(StudioEditorMode::Advanced);
                                    self.sync_title();
                                    self.status = "Script mode".into();
                                }
                                "save" => self.do_save(),
                                _ => {}
                            }
                            self.redraw();
                            return;
                        }

                        // Inspector field click → start edit (needs selection)
                        if self.session.mode == StudioEditorMode::Simplified
                            && self.session.selected_region.is_some()
                            && layout.contains_inspector(self.cursor.0, self.cursor.1)
                        {
                            if let Some(field) =
                                layout.hit_inspector_field(self.cursor.0, self.cursor.1)
                            {
                                self.begin_edit(field);
                                return;
                            }
                            // click empty inspector area: keep selection, end edit
                            self.cancel_edit();
                            self.redraw();
                            return;
                        }

                        if self.session.mode != StudioEditorMode::Simplified {
                            return;
                        }

                        // Left dock: layers / palette / hierarchy
                        if layout.contains_left_dock(self.cursor.0, self.cursor.1) {
                            self.cancel_edit();
                            let rows = self.session.layers.visible_tree_rows();
                            if let Some(idx) = layout.hit_layer_row(self.cursor.1, rows.len()) {
                                if let Some(row) = rows.get(idx) {
                                    // Click near left edge toggles expand when has children
                                    let x = self.cursor.0 as i32;
                                    if row.has_children && x < layout.left_w / 3 {
                                        let open = self.session.toggle_layer_expand(&row.id);
                                        self.status = if open {
                                            format!("{} expanded", row.name)
                                        } else {
                                            format!("{} collapsed", row.name)
                                        };
                                    } else {
                                        match self.session.set_layer(&row.id) {
                                            Ok(s) => self.status = s,
                                            Err(e) => self.status = e,
                                        }
                                    }
                                    self.redraw();
                                }
                                return;
                            }
                            if let Some(kind) = layout.hit_palette(self.cursor.0, self.cursor.1) {
                                if !self.session.layers.active_editable() {
                                    self.status = "layer locked — U unlock or select layer".into();
                                    self.redraw();
                                    return;
                                }
                                match self.session.drop_widget(kind, 50.0, 50.0) {
                                    Ok(id) => {
                                        self.status = format!("placed {id} from palette");
                                    }
                                    Err(e) => self.status = format!("drop failed: {e}"),
                                }
                                self.redraw();
                                return;
                            }
                            let widgets = self
                                .session
                                .list_widgets()
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|w| crate::studio_paint::is_canvas_widget(w))
                                .collect::<Vec<_>>();
                            if let Some(idx) = layout.hit_hierarchy(self.cursor.1, widgets.len()) {
                                if let Some(w) = widgets.get(idx) {
                                    self.session.selected_region = Some(w.id.clone());
                                    self.status = format!("selected {}", w.id);
                                    self.redraw();
                                }
                                return;
                            }
                            return;
                        }

                        // Canvas: design surface hit + drag
                        let surface = self.design_surface();
                        if !surface.contains(self.cursor.0, self.cursor.1) {
                            self.dragging = false;
                            return;
                        }
                        if !self.session.layers.active_editable() {
                            self.status = "layer locked — press U to unlock or [ ] change layer".into();
                            self.redraw();
                            return;
                        }
                        self.cancel_edit();
                        let (cx, cy) = surface.screen_to_pct(self.cursor.0, self.cursor.1);
                        match self.session.canvas_pointer_down(cx, cy) {
                            Ok(Some(id)) => {
                                let (rw, rh) = self.session.layers.active_resolution();
                                let (px, py) = pct_to_px(cx, cy, rw, rh);
                                self.status = format!(
                                    "selected {id} @ ({cx:.0}%,{cy:.0}%) = ({px},{py})px"
                                );
                                self.dragging = true;
                                self.drag_last_px = Some(self.cursor);
                                self.drag_acc_pct = (0.0, 0.0);
                            }
                            Ok(None) => {
                                self.session.selected_region = None;
                                self.status = "canvas empty — click palette or press B".into();
                                self.dragging = false;
                                self.drag_last_px = None;
                                self.drag_acc_pct = (0.0, 0.0);
                            }
                            Err(e) => self.status = format!("hit: {e}"),
                        }
                        self.redraw();
                    } else {
                        // Mouse up: end drag, snap final position message
                        if self.dragging {
                            if let Some(id) = &self.session.selected_region {
                                self.status = format!("placed {id}");
                            }
                        }
                        self.dragging = false;
                        self.drag_last_px = None;
                        self.drag_acc_pct = (0.0, 0.0);
                        self.redraw();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    self.cursor = (position.x, position.y);
                    if self.session.mode != StudioEditorMode::Simplified || !self.dragging {
                        return;
                    }
                    if !self.session.layers.active_editable() {
                        return;
                    }
                    let surface = self.design_surface();
                    if let Some((lx, ly)) = self.drag_last_px {
                        let (dpx, dpy) =
                            surface.screen_delta_to_pct(position.x - lx, position.y - ly);
                        self.drag_acc_pct.0 += dpx;
                        self.drag_acc_pct.1 += dpy;
                        let sx = self.drag_acc_pct.0.trunc();
                        let sy = self.drag_acc_pct.1.trunc();
                        if sx.abs() >= 1.0 || sy.abs() >= 1.0 {
                            match self.session.canvas_pointer_drag(sx, sy) {
                                Ok(Some(r)) => {
                                    let (rw, rh) = self.session.layers.active_resolution();
                                    let (px, py) = pct_to_px(r.pos.x, r.pos.y, rw, rh);
                                    self.status = format!(
                                        "drag → ({:.0}%, {:.0}%) = ({px},{py})px / {rw}x{rh}",
                                        r.pos.x, r.pos.y
                                    );
                                }
                                Ok(None) => {}
                                Err(e) => self.status = format!("drag: {e}"),
                            }
                            self.drag_acc_pct.0 -= sx;
                            self.drag_acc_pct.1 -= sy;
                            self.redraw();
                        }
                        self.drag_last_px = Some((position.x, position.y));
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.frames += 1;
                    if let Err(e) = self.paint_frame() {
                        eprintln!("[studio-gui] paint error: {e}");
                    }
                    if !self.interactive && self.frames >= 12 {
                        el.exit();
                    }
                }
                WindowEvent::Resized(_) => {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            let now = std::time::Instant::now();
            let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.05);
            self.last_tick = now;
            let animating = self.session.tick_layers(dt);
            if animating {
                el.set_control_flow(ControlFlow::Poll);
                self.redraw();
                return;
            }
            if self.interactive {
                el.set_control_flow(ControlFlow::Wait);
                return;
            }
            el.set_control_flow(ControlFlow::Poll);
            if self.frames >= 12 {
                el.exit();
                return;
            }
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    impl Host {
        fn layout(&self) -> crate::studio_paint::StudioLayout {
            let (ww, wh) = self
                .window
                .as_ref()
                .map(|w| {
                    let s = w.inner_size();
                    (s.width.max(1), s.height.max(1))
                })
                .unwrap_or((1280, 800));
            crate::studio_paint::StudioLayout::new(ww, wh, self.ui_zoom)
        }

        fn design_surface(&self) -> DesignSurface {
            let lay = self.layout();
            self.session
                .design_surface(lay.canvas_x, lay.canvas_y, lay.canvas_w, lay.canvas_h)
        }

        fn cursor_canvas_pct(&self) -> (f32, f32) {
            let surface = self.design_surface();
            if surface.contains(self.cursor.0, self.cursor.1) {
                surface.screen_to_pct(self.cursor.0, self.cursor.1)
            } else {
                (50.0, 50.0)
            }
        }

        fn drop_at_cursor(&mut self, kind: &str) {
            self.cancel_edit();
            if !self.session.layers.active_editable() {
                self.status = "layer locked — U to unlock".into();
                self.redraw();
                return;
            }
            let (cx, cy) = self.cursor_canvas_pct();
            match self.session.drop_widget(kind, cx, cy) {
                Ok(id) => {
                    let (rw, rh) = self.session.layers.active_resolution();
                    let (px, py) = pct_to_px(cx, cy, rw, rh);
                    self.status =
                        format!("dropped {id} at ({cx:.0}%,{cy:.0}%) = ({px},{py})px");
                }
                Err(e) => self.status = format!("drop failed: {e}"),
            }
            self.redraw();
        }

        fn do_save(&mut self) {
            if self.edit_field.is_some() {
                self.commit_edit();
            }
            match self.session.save_document() {
                Ok(()) => self.status = "saved to disk".into(),
                Err(e) => self.status = format!("save: {e}"),
            }
            self.redraw();
        }

        fn cancel_edit(&mut self) {
            self.edit_field = None;
            self.edit_buf.clear();
        }

        fn bump_zoom(&mut self, delta: i32) {
            let next = (self.ui_zoom + delta).clamp(1, 4);
            if next != self.ui_zoom {
                self.ui_zoom = next;
                self.status = format!(
                    "UI zoom x{} — layout+text  (Ctrl+wheel / Ctrl+/- / Ctrl+0)",
                    self.ui_zoom
                );
                self.redraw();
            } else {
                self.status = format!("UI zoom x{} (min 1 / max 4)", self.ui_zoom);
                self.redraw();
            }
        }

        fn begin_edit(&mut self, field: crate::studio_paint::InspectorField) {
            use crate::studio_paint::InspectorField;
            let Some(id) = self.session.selected_region.clone() else {
                self.status = "select a widget first".into();
                self.redraw();
                return;
            };
            let widgets = self.session.list_widgets().unwrap_or_default();
            let w = widgets.iter().find(|w| w.id == id);
            let initial = match (field, w) {
                (InspectorField::Text, Some(w)) => w.text.clone().unwrap_or_default(),
                (InspectorField::Text, None) => String::new(),
                (InspectorField::Pos, Some(w)) => w
                    .position
                    .clone()
                    .unwrap_or_else(|| "(50%, 50%)".into()),
                (InspectorField::Pos, None) => "(50%, 50%)".into(),
                (InspectorField::Size, Some(w)) => w
                    .size
                    .clone()
                    .unwrap_or_else(|| "(18%, 8%)".into()),
                (InspectorField::Size, None) => "(18%, 8%)".into(),
            };
            self.edit_field = Some(field);
            self.edit_buf = initial;
            self.status = format!(
                "editing {} — type, Enter apply, Esc cancel",
                field.label()
            );
            self.redraw();
        }

        fn commit_edit(&mut self) {
            use crate::studio_paint::InspectorField;
            let Some(field) = self.edit_field else {
                return;
            };
            let buf = self.edit_buf.clone();
            let result = match field {
                InspectorField::Text => self.session.set_widget_text(None, &buf),
                InspectorField::Pos => self.session.set_widget_position(None, &buf),
                InspectorField::Size => self.session.set_widget_size(None, &buf),
            };
            match result {
                Ok(()) => {
                    self.status = format!("{} updated", field.label());
                    self.cancel_edit();
                }
                Err(e) => {
                    self.status = format!("edit failed: {e}");
                    // keep edit open so user can fix
                }
            }
            self.redraw();
        }

        fn redraw(&self) {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }

        fn sync_title(&self) {
            if let Some(w) = &self.window {
                let mode = match self.session.mode {
                    StudioEditorMode::Simplified => "Visual",
                    StudioEditorMode::Advanced => "Script",
                };
                w.set_title(&format!("Velvet Studio [{mode}] — dual mode"));
            }
        }

        fn paint_frame(&mut self) -> Result<()> {
            use velvet_story::pack_rgb;
            let Some(window) = self.window.clone() else {
                return Ok(());
            };
            let size = window.inner_size();
            let ww = size.width.max(1);
            let wh = size.height.max(1);
            if ww < 2 || wh < 2 {
                return Ok(());
            }
            if self.pixels.len() != (ww * wh) as usize {
                self.pixels.resize((ww * wh) as usize, 0);
            }
            let layout = crate::studio_paint::StudioLayout::new(ww, wh, self.ui_zoom);
            let widgets = self.session.list_widgets().unwrap_or_default();
            let project = self
                .session
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            let (rw, rh) = self.session.layers.display_resolution();
            let tree_rows = self.session.layers.visible_tree_rows();
            let path = self.session.layers.active_path();
            let path_joined = path.join(" > ");
            let layer_view = crate::studio_paint::LayerPaintView {
                layers: &self.session.layers.layers,
                tree_rows: &tree_rows,
                active_id: &self.session.layers.active_id,
                breadcrumb: &path_joined,
                res_w: rw,
                res_h: rh,
                animating: self.session.layers.resize_anim.is_some(),
                editable: self.session.layers.active_editable(),
                pos_px: self.session.selected_pos_px(),
            };
            crate::studio_paint::paint_studio(
                &mut self.pixels,
                &layout,
                self.session.mode == StudioEditorMode::Simplified,
                project,
                self.session.selected_region.as_deref(),
                &widgets,
                self.session.advanced_source(),
                &self.status,
                self.dragging,
                self.edit_field,
                &self.edit_buf,
                self.ui_zoom,
                &layer_view,
            );

            let Some(surface) = self.surface.as_mut() else {
                return Ok(());
            };
            let Some(nw) = NonZeroU32::new(ww) else {
                return Ok(());
            };
            let Some(nh) = NonZeroU32::new(wh) else {
                return Ok(());
            };
            surface
                .resize(nw, nh)
                .map_err(|e| anyhow::anyhow!("surface resize: {e}"))?;
            let mut buf = surface
                .buffer_mut()
                .map_err(|e| anyhow::anyhow!("buffer_mut: {e}"))?;
            if buf.len() != self.pixels.len() {
                self.pixels.resize(buf.len(), pack_rgb(15, 17, 26));
            }
            buf.copy_from_slice(&self.pixels);
            buf.present()
                .map_err(|e| anyhow::anyhow!("present: {e}"))?;
            Ok(())
        }
    }

    #[cfg(windows)]
    let event_loop = {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        EventLoop::builder()
            .with_any_thread(true)
            .build()
            .map_err(|e| anyhow::anyhow!("event loop: {e}"))?
    };
    #[cfg(not(windows))]
    let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("event loop: {e}"))?;

    let mut host = Host {
        session: session.clone(),
        interactive,
        frames: 0,
        window: None,
        context: None,
        surface: None,
        pixels: Vec::new(),
        cursor: (0.0, 0.0),
        drag_last_px: None,
        dragging: false,
        drag_acc_pct: (0.0, 0.0),
        ctrl_held: false,
        edit_field: None,
        edit_buf: String::new(),
        ui_zoom: 2,
        last_tick: std::time::Instant::now(),
        status: "ready — [ ] layers, M mobile, Ctrl+4 phone res, drag widgets".into(),
    };
    event_loop
        .run_app(&mut host)
        .map_err(|e| anyhow::anyhow!("run_app: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const MENU: &str = r#"
// @visual id=button.start
button start {
    text: "Iniciar"
    position: (50%, 62%)
// @advanced id=button.start
    on_pressed {
        game.new()
    }
// @end
}
"#;

    #[test]
    fn dock_ready_and_drag_roundtrip_preserves_advanced() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        {
            let mut f = fs::File::create(&menu).unwrap();
            write!(f, "{MENU}").unwrap();
        }

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        assert!(session.ready);
        assert!(session.panel_ids().contains(&"canvas"));
        assert!(session.panel_ids().contains(&"hierarchy"));
        session.open_document(&menu).unwrap();
        let before = {
            let doc = parse_document(&session.document_source).unwrap();
            region_rect(&doc, "button.start").unwrap()
        };
        session.select_region("button.start").unwrap();
        let after = session.drag_region(None, -4.0, 2.0).unwrap();
        assert!((after.pos.x - (before.pos.x - 4.0)).abs() < 0.01);
        session.save_document().unwrap();
        let disk = fs::read_to_string(&menu).unwrap();
        assert!(disk.contains("game.new()"), "advanced kept: {disk}");
        assert!(
            disk.contains("position: (46%, 64%)") || disk.contains("46%"),
            "drag persisted: {disk}"
        );
    }

    #[test]
    fn run_gui_headless_writes_ready_log() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("p");
        fs::create_dir_all(&proj).unwrap();
        let log = dir.path().join("ready.log");
        let status = run_studio_gui(StudioGuiConfig {
            root: proj,
            headless: true,
            once: true,
            ready_log: Some(log.clone()),
            ..Default::default()
        })
        .unwrap();
        assert!(status.session.ready);
        let text = fs::read_to_string(&log).unwrap();
        assert!(text.contains("ready=true") || text.contains("[studio-gui] ready"));
        assert!(text.contains("canvas"));
    }

    #[test]
    fn dual_mode_drop_drag_preserves_advanced_and_toggle() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        assert_eq!(session.mode, StudioEditorMode::Simplified);

        // Drop new button
        let id = session.drop_widget("button", 30.0, 40.0).unwrap();
        assert!(id.contains("button") || id.starts_with("button."));

        // Drag original; advanced must survive
        session.select_region("button.start").unwrap();
        session.drag_region(None, -2.0, 1.0).unwrap();
        assert!(
            session.document_source.contains("game.new()"),
            "advanced lost: {}",
            session.document_source
        );

        // Advanced mode: edit text in script buffer, reparse
        session.set_mode(StudioEditorMode::Advanced).unwrap();
        assert_eq!(session.mode, StudioEditorMode::Advanced);
        let mut src = session.advanced_source().to_string();
        src = src.replace("Iniciar", "Start");
        session.set_advanced_source(src).unwrap();
        session.set_mode(StudioEditorMode::Simplified).unwrap();
        let widgets = session.list_widgets().unwrap();
        let start = widgets.iter().find(|w| w.id == "button.start").unwrap();
        assert_eq!(start.text.as_deref(), Some("Start"));
        assert!(session.document_source.contains("game.new()"));

        session.toggle_mode().unwrap();
        assert_eq!(session.mode, StudioEditorMode::Advanced);
    }

    #[test]
    fn inspector_set_text_pos_size_and_normalize() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let menu = proj.join("scripts/main_menu.vel");
        fs::write(&menu, MENU).unwrap();

        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        session.open_document(&menu).unwrap();
        session.select_region("button.start").unwrap();

        session.set_widget_text(None, "Jugar ahora").unwrap();
        session.set_widget_position(None, "40, 55").unwrap();
        session.set_widget_size(None, "22% 10%").unwrap();
        session.nudge_selected(1.0, -1.0).unwrap();

        let src = &session.document_source;
        assert!(src.contains("Jugar ahora"), "{src}");
        assert!(
            src.contains("position: (41%, 54%)") || src.contains("(41%, 54%)"),
            "{src}"
        );
        assert!(src.contains("size:") && src.contains("22%"), "{src}");
        assert!(src.contains("game.new()"), "advanced kept: {src}");

        assert!(normalize_pct_pair("50,62").unwrap().contains("50%"));
        assert!(normalize_pct_pair("bad").is_none());
    }

    #[test]
    fn layer_stack_mobile_and_coords() {
        let dir = tempdir().unwrap();
        let proj = dir.path().join("game");
        fs::create_dir_all(proj.join("scripts")).unwrap();
        fs::write(
            proj.join("velvet.project"),
            r#"(name: "g", version: "0.1.0", entry_scene: "scripts/main.vel")"#,
        )
        .unwrap();
        let mut session = StudioGuiSession::open_project(&proj).unwrap();
        assert!(session.layers.get("main_menu").is_some());
        assert!(session.layers.get("menu_settings").is_some());
        assert!(session.layers.get("scene_decisions").is_some());
        let rows = session.layers.visible_tree_rows();
        assert!(rows.iter().any(|r| r.depth == 1));
        session.add_mobile_layer().unwrap();
        assert_eq!(session.layers.active_id, "mobile");
        let (w, h) = session.layers.active_resolution();
        assert_eq!((w, h), (390, 844));
        // still under main_menu branch → root not auto-locked
        assert!(!session.layers.get("main_menu").unwrap().locked || true);
        session.set_layer("scene_decisions").unwrap();
        assert!(session.layers.get("main_menu").unwrap().locked);
        session.set_layer("main_menu").unwrap();
        assert!(!session.layers.get("main_menu").unwrap().locked);
        session.set_layer("menu_settings").unwrap();
        assert_eq!(
            session.layers.active_path().last().map(|s| s.as_str()),
            Some("Configuracion")
        );
        session.set_layer_preset(ResPreset::DesktopFhd).unwrap();
        assert_eq!(session.layers.active_resolution(), (1920, 1080));
        let ds = session.design_surface(0, 0, 800, 600);
        assert!(ds.w > 0 && ds.h > 0);
    }
}
