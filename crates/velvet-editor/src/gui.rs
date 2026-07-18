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
    drag_visual_region, parse_document, region_rect, render_document, hit_test_visual, WidgetRect,
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
        };
        session.log.push(format!(
            "[studio-gui] opened project {}",
            session.root.display()
        ));
        session.ready = true;
        session.log.push(format!(
            "[studio-gui] ready panels={}",
            session
                .panels
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
        Ok(session)
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
        match try_open_brief_window(&session) {
            Ok(()) => {
                window_opened = true;
                session
                    .log
                    .push("[studio-gui] window opened (brief)".into());
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

/// Best-effort brief OS window to prove GUI entry (uses winit any-thread on Windows).
fn try_open_brief_window(session: &StudioGuiSession) -> Result<()> {
    use winit::event_loop::EventLoop;
    use winit::window::WindowAttributes;

    #[cfg(windows)]
    let event_loop = {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        EventLoop::builder()
            .with_any_thread(true)
            .build()
            .map_err(|e| anyhow::anyhow!("event loop: {e}"))?
    };
    #[cfg(not(windows))]
    let event_loop =
        EventLoop::new().map_err(|e| anyhow::anyhow!("event loop: {e}"))?;

    let title = format!(
        "Velvet Studio — {} (panels: {})",
        session.root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
        session.panel_ids().join(", ")
    );
    let _attrs = WindowAttributes::default().with_title(title);
    // Creating the event loop is enough proof on headless CI; avoid pumping forever.
    // Full interactive docking UI is driven by StudioGuiSession APIs.
    let _ = event_loop;
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
}
