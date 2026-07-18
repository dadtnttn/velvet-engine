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
    drag_visual_region, parse_document, region_rect, render_document, hit_test_visual,
    DesignerWidget, UiDesigner, WidgetRect,
};

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
        };
        session.log.push(format!(
            "[studio-gui] opened project {}",
            session.root.display()
        ));
        session.ready = true;
        session.log.push(format!(
            "[studio-gui] ready panels={} mode=simplified",
            session
                .panels
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
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
        drag_last: Option<(f64, f64)>,
        dragging: bool,
    }

    impl ApplicationHandler for Host {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let mode = match self.session.mode {
                StudioEditorMode::Simplified => "Simplified",
                StudioEditorMode::Advanced => "Advanced",
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
                .with_inner_size(LogicalSize::new(1100.0, 700.0));
            let window = Arc::new(el.create_window(attrs).expect("window"));
            let context = SbContext::new(window.clone()).expect("ctx");
            let surface = Surface::new(&context, window.clone()).expect("surface");
            self.context = Some(context);
            self.surface = Some(surface);
            self.window = Some(window);
        }

        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
            match ev {
                WindowEvent::CloseRequested => el.exit(),
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }
                    let PhysicalKey::Code(c) = event.physical_key else {
                        return;
                    };
                    match c {
                        KeyCode::Tab => {
                            let _ = self.session.toggle_mode();
                            if let Some(w) = &self.window {
                                let mode = match self.session.mode {
                                    StudioEditorMode::Simplified => "Simplified",
                                    StudioEditorMode::Advanced => "Advanced",
                                };
                                w.set_title(&format!(
                                    "Velvet Studio [{mode}] — dual mode"
                                ));
                            }
                        }
                        KeyCode::KeyS if self.session.mode == StudioEditorMode::Simplified => {
                            // Drop a button at center
                            let _ = self.session.drop_widget("button", 50.0, 50.0);
                        }
                        KeyCode::Digit1 => {
                            let _ = self.session.set_mode(StudioEditorMode::Simplified);
                        }
                        KeyCode::Digit2 => {
                            let _ = self.session.set_mode(StudioEditorMode::Advanced);
                        }
                        KeyCode::KeyW if self.session.selected_region.is_some() => {
                            let _ = self.session.save_document();
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
                    if self.session.mode != StudioEditorMode::Simplified {
                        return;
                    }
                    self.dragging = state == ElementState::Pressed;
                    if !self.dragging {
                        self.drag_last = None;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if self.session.mode != StudioEditorMode::Simplified {
                        return;
                    }
                    let Some(w) = &self.window else { return };
                    let size = w.inner_size();
                    let px = position.x / size.width.max(1) as f64 * 100.0;
                    let py = position.y / size.height.max(1) as f64 * 100.0;
                    if self.dragging {
                        if let Some((lx, ly)) = self.drag_last {
                            let dx = (px - lx) as f32;
                            let dy = (py - ly) as f32;
                            let _ = self.session.canvas_pointer_drag(dx, dy);
                        } else {
                            let _ = self.session.canvas_pointer_down(px as f32, py as f32);
                        }
                        self.drag_last = Some((px, py));
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.frames += 1;
                    self.paint_frame();
                    if !self.interactive && self.frames >= 12 {
                        el.exit();
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            el.set_control_flow(ControlFlow::Poll);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            if !self.interactive && self.frames >= 12 {
                el.exit();
            }
        }
    }

    impl Host {
        fn paint_frame(&mut self) {
            use velvet_story::{draw_text_line, fill_rect, pack_rgb};
            let Some(window) = self.window.clone() else {
                return;
            };
            let size = window.inner_size();
            let ww = size.width.max(1);
            let wh = size.height.max(1);
            if self.pixels.len() != (ww * wh) as usize {
                self.pixels.resize((ww * wh) as usize, 0);
            }
            // chrome
            fill_rect(
                &mut self.pixels,
                ww,
                wh,
                0,
                0,
                ww as i32,
                wh as i32,
                pack_rgb(22, 18, 32),
            );
            // left dock
            fill_rect(
                &mut self.pixels,
                ww,
                wh,
                0,
                0,
                (ww as f32 * 0.18) as i32,
                wh as i32,
                pack_rgb(28, 24, 40),
            );
            // right dock
            fill_rect(
                &mut self.pixels,
                ww,
                wh,
                (ww as f32 * 0.78) as i32,
                0,
                ww as i32,
                wh as i32,
                pack_rgb(28, 24, 40),
            );
            // bottom
            fill_rect(
                &mut self.pixels,
                ww,
                wh,
                0,
                (wh as f32 * 0.82) as i32,
                ww as i32,
                wh as i32,
                pack_rgb(18, 16, 28),
            );

            let mode = match self.session.mode {
                StudioEditorMode::Simplified => "SIMPLIFIED (Tab=advanced, S=drop button, drag=move)",
                StudioEditorMode::Advanced => "ADVANCED (Tab=simplified; script buffer in session)",
            };
            draw_text_line(
                &mut self.pixels,
                ww,
                wh,
                12,
                10,
                mode,
                pack_rgb(220, 200, 255),
                2,
            );
            draw_text_line(
                &mut self.pixels,
                ww,
                wh,
                12,
                36,
                &format!(
                    "panels={} sel={}",
                    self.session.panel_ids().join(","),
                    self.session
                        .selected_region
                        .as_deref()
                        .unwrap_or("-")
                ),
                pack_rgb(160, 155, 180),
                1,
            );

            let cx0 = (ww as f32 * 0.20) as i32;
            let cy0 = (wh as f32 * 0.12) as i32;
            let cw = (ww as f32 * 0.56) as i32;
            let ch = (wh as f32 * 0.66) as i32;
            fill_rect(
                &mut self.pixels,
                ww,
                wh,
                cx0,
                cy0,
                cx0 + cw,
                cy0 + ch,
                pack_rgb(12, 10, 20),
            );

            if self.session.mode == StudioEditorMode::Simplified {
                if let Ok(widgets) = self.session.list_widgets() {
                    for w in widgets {
                        let (x, y) = parse_pct_pair(w.position.as_deref().unwrap_or("(50%,50%)"));
                        let (sw, sh) = parse_pct_pair(w.size.as_deref().unwrap_or("(18%,8%)"));
                        let px = cx0 + (x / 100.0 * cw as f32) as i32 - 40;
                        let py = cy0 + (y / 100.0 * ch as f32) as i32 - 16;
                        let bw = (sw / 100.0 * cw as f32).max(48.0) as i32;
                        let bh = (sh / 100.0 * ch as f32).max(28.0) as i32;
                        let sel = self.session.selected_region.as_deref() == Some(w.id.as_str());
                        let col = if sel {
                            pack_rgb(80, 120, 200)
                        } else {
                            pack_rgb(60, 50, 90)
                        };
                        fill_rect(
                            &mut self.pixels,
                            ww,
                            wh,
                            px,
                            py,
                            px + bw,
                            py + bh,
                            col,
                        );
                        let label = w.text.as_deref().unwrap_or(w.id.as_str());
                        draw_text_line(
                            &mut self.pixels,
                            ww,
                            wh,
                            px + 6,
                            py + 8,
                            label,
                            pack_rgb(240, 240, 250),
                            1,
                        );
                    }
                }
            } else {
                // Advanced: show first lines of script
                let lines: Vec<&str> = self.session.document_source.lines().take(24).collect();
                for (i, line) in lines.iter().enumerate() {
                    let clipped: String = line.chars().take(70).collect();
                    draw_text_line(
                        &mut self.pixels,
                        ww,
                        wh,
                        cx0 + 8,
                        cy0 + 12 + i as i32 * 14,
                        &clipped,
                        pack_rgb(180, 220, 180),
                        1,
                    );
                }
            }

            let Some(surface) = self.surface.as_mut() else {
                return;
            };
            let _ = surface.resize(
                NonZeroU32::new(ww).unwrap(),
                NonZeroU32::new(wh).unwrap(),
            );
            let mut buf = surface.buffer_mut().unwrap();
            let n = self.pixels.len().min(buf.len());
            buf[..n].copy_from_slice(&self.pixels[..n]);
            let _ = buf.present();
        }
    }

    fn parse_pct_pair(s: &str) -> (f32, f32) {
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
        drag_last: None,
        dragging: false,
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
}
